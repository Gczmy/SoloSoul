//! LLM usage statistics screen.

use crate::i18n::I18n;
use crate::t;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use solosoul_core::llm::config::LlmUsageStats;

pub fn render(frame: &mut Frame, area: Rect, stats: &LlmUsageStats, selected: usize, i18n: &I18n) {
    let layout = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Text::from(vec![
        Line::from(vec![Span::styled(
            t!(i18n, "llm-stats-title"),
            Style::default().bold().fg(Color::Cyan),
        )]),
        Line::from(t!(i18n, "hint-esc-q")),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    // Summary
    let summary = Paragraph::new(Text::from(vec![Line::from(vec![Span::raw(t!(
        i18n,
        "llm-stats-summary",
        count = &stats.usage_count.to_string(),
        tokens = &stats.total_tokens.to_string(),
        prompt = &stats.prompt_tokens.to_string(),
        completion = &stats.completion_tokens.to_string()
    ))])]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(summary, layout[1]);

    // Per-model table
    let header_values = [
        t!(i18n, "llm-stats-model"),
        t!(i18n, "llm-stats-provider"),
        t!(i18n, "llm-stats-count"),
        t!(i18n, "llm-stats-tokens"),
    ];
    let header_cells = header_values
        .iter()
        .map(|h| Cell::from(h.as_str()).style(Style::default().bold()));
    let header_row = Row::new(header_cells).height(1);
    let rows: Vec<Row> = stats
        .per_model_stats
        .iter()
        .map(|m| {
            Row::new(vec![
                Cell::from(m.model.as_str()),
                Cell::from(m.provider.as_str()),
                Cell::from(m.count.to_string()),
                Cell::from(m.tokens.to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ],
    )
    .header(header_row)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(t!(i18n, "llm-stats-by-model")),
    );

    let mut table_state = TableState::default();
    table_state.select(Some(selected));
    frame.render_stateful_widget(table, layout[2], &mut table_state);
}
