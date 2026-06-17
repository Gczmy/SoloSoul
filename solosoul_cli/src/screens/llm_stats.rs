//! LLM usage statistics screen.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;
use solosoul_core::llm::config::LlmUsageStats;

pub fn render(frame: &mut Frame, area: Rect, stats: &LlmUsageStats, selected: usize) {
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
            "LLM 使用统计",
            Style::default().bold().fg(Color::Cyan),
        )]),
        Line::from("Esc/q 返回"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, layout[0]);

    // Summary
    let summary = Paragraph::new(Text::from(vec![Line::from(vec![Span::raw(format!(
        "总请求: {}  |  总 tokens: {}  |  Prompt: {}  |  Completion: {}",
        stats.usage_count, stats.total_tokens, stats.prompt_tokens, stats.completion_tokens
    ))])]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(summary, layout[1]);

    // Per-model table
    let header_cells = ["模型", "提供商", "次数", "Tokens"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().bold()));
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
    .block(Block::default().borders(Borders::ALL).title("按模型"));

    let mut table_state = TableState::default();
    table_state.select(Some(selected));
    frame.render_stateful_widget(table, layout[2], &mut table_state);
}
