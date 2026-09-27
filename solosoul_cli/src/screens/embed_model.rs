//! /embed_model 模型列表屏：列出已安装/激活情况。

use crate::i18n::I18n;
use crate::t;
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

#[derive(Debug, Clone)]
pub struct EmbedDownloadView {
    pub model_id: String,
    pub current: u64,
    pub total: Option<u64>,
    pub cancelling: bool,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    models: &[EmbedModelEntry],
    info: &str,
    downloads: &[EmbedDownloadView],
    i18n: &I18n,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            t!(i18n, "embed-model-title"),
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

    if models.is_empty() && downloads.is_empty() {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled(
                t!(i18n, "embed-model-not-installed"),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("\n"),
            Span::raw(t!(i18n, "embed-model-install-hint")),
        ]))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!(i18n, "embed-model-list-title")),
        );
        frame.render_widget(empty, chunks[1]);
    } else {
        let mut items: Vec<ListItem> = models
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
        for download in downloads {
            let progress = match download.total {
                Some(total) => format!("{} / {} B", download.current, total),
                None => format!("{} B", download.current),
            };
            let state = if download.cancelling {
                t!(i18n, "embed-model-cancelling")
            } else {
                t!(i18n, "embed-model-downloading")
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    &download.model_id,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(state, Style::default().fg(Color::Yellow)),
                Span::raw("  "),
                Span::styled(progress, Style::default().fg(Color::Magenta)),
            ])));
        }
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!(i18n, "embed-model-list-title")),
        );
        frame.render_widget(list, chunks[1]);
    }

    let hint = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                t!(i18n, "ocr-hint-prefix"),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(": "),
            Span::raw(t!(i18n, "embed-model-registry-hint")),
            Span::raw(" "),
            Span::styled(
                "SOLOSOUL_EMBED_REGISTRY",
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("。"),
        ]),
        Line::from(t!(i18n, "embed-model-cancel-hint")),
    ]);
    frame.render_widget(hint, chunks[2]);
}
