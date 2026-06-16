//! Conversation history list screen.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;
use solosoul_core::llm::config::ConversationSummary;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    conversations: &[ConversationSummary],
    selected: usize,
) {
    let layout = Layout::default()
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // Header
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::styled(
            "LLM 对话历史",
            Style::default().bold().fg(Color::Cyan),
        )]),
        Line::from(format!(
            "共 {} 条对话  |  ↑↓ 选择  Enter 打开  Esc/q 返回",
            conversations.len()
        )),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    // Conversation list
    if conversations.is_empty() {
        let empty = Paragraph::new("暂无对话记录")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(empty, layout[1]);
        return;
    }

    let items: Vec<ListItem> = conversations
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let style = if i == selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            // Truncate date
            let date = if c.updated_at.len() > 16 {
                &c.updated_at[..16]
            } else {
                &c.updated_at
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}  ", c.name),
                    style.fg(Color::White).bold(),
                ),
                Span::styled(
                    format!("({} 条消息)", c.message_count),
                    style.fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("  {}", date),
                    style.fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("对话"));
    frame.render_stateful_widget(list, layout[1], &mut list_state);
}
