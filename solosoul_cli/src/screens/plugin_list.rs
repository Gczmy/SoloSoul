//! 插件列表 TUI 屏幕。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::commands::plugin::PluginSummary;
use crate::i18n::I18n;
use crate::t;

/// 渲染插件列表页。
pub fn render(
    frame: &mut Frame,
    area: Rect,
    plugins: &[PluginSummary],
    selected: usize,
    filter: &str,
    i18n: &I18n,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let filtered: Vec<&PluginSummary> = plugins
        .iter()
        .filter(|p| {
            filter.is_empty()
                || p.name.to_lowercase().contains(&filter.to_lowercase())
                || p.description
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
        })
        .collect();

    let title = if filter.is_empty() {
        format!(
            "{} ({})",
            t!(
                i18n,
                "plugin-list-title",
                count = &plugins.len().to_string()
            ),
            plugins.len()
        )
    } else {
        format!(
            "{}: \"{}\" ({} / {})",
            t!(
                i18n,
                "plugin-list-title",
                count = &plugins.len().to_string()
            ),
            filter,
            filtered.len(),
            plugins.len()
        )
    };

    let items: Vec<ListItem> = if filtered.is_empty() {
        let msg = if filter.is_empty() {
            t!(i18n, "plugin-list-empty")
        } else {
            t!(i18n, "plugin-list-no-match")
        };
        vec![ListItem::new(msg)]
    } else {
        filtered
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let prefix = if i == selected { "> " } else { "  " };
                let style = if i == selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![Span::styled(
                    format!("{}{} v{}  [{}]", prefix, p.name, p.version, p.tier),
                    style,
                )]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(Style::default());

    frame.render_widget(list, chunks[0]);

    // 底部帮助
    let hint_text = if filter.is_empty() {
        t!(i18n, "plugin-list-hint")
    } else {
        t!(i18n, "plugin-list-hint-filtering")
    };
    let help = Paragraph::new(Line::from(vec![Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    )]));

    frame.render_widget(help, chunks[1]);
}
