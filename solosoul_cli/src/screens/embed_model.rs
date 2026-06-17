//! /embed_model 模型列表屏：列出已安装/激活情况。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug, Clone)]
pub struct EmbedModelEntry {
    pub id: String,
    pub installed: bool,
    pub size_mb: f32,
    pub source: String,
}

pub fn render(frame: &mut Frame, area: Rect, models: &[EmbedModelEntry], info: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "Embedding 模型",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(info, Style::default().fg(Color::DarkGray)),
    ]))
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    if models.is_empty() {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled(
                "(本地尚未安装 embedding 模型)",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("\n"),
            Span::raw("用 `/embed_model install <id>` 从注册表下载并安装。"),
        ]))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("模型列表"));
        frame.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = models
            .iter()
            .map(|m| {
                let line = Line::from(vec![
                    Span::styled(&m.id, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:.1} MB", m.size_mb),
                        Style::default().fg(Color::Magenta),
                    ),
                    Span::raw("  "),
                    Span::styled(&m.source, Style::default().fg(Color::DarkGray)),
                ]);
                ListItem::new(line)
            })
            .collect();
        let list = List::new(items).block(Block::default().borders(Borders::ALL).title("模型列表"));
        frame.render_widget(list, chunks[1]);
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("提示", Style::default().fg(Color::Cyan)),
        Span::raw(": 注册表 URL 由环境变量 "),
        Span::styled(
            "SOLOSOUL_EMBED_REGISTRY",
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" 控制。"),
    ]));
    frame.render_widget(hint, chunks[2]);
}
