//! 插件列表 TUI 屏幕。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::commands::plugin::PluginSummary;

/// 渲染插件列表页。
pub fn render(
    frame: &mut Frame,
    area: Rect,
    plugins: &[PluginSummary],
    selected: usize,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let items: Vec<ListItem> = if plugins.is_empty() {
        vec![ListItem::new("暂无可用插件")]
    } else {
        plugins
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let prefix = if i == selected { "> " } else { "  " };
                let style = if i == selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{}{} v{}  [{}]", prefix, p.name, p.version, p.tier),
                        style,
                    ),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("插件列表 (共 {} 个)", plugins.len()))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default());

    frame.render_widget(list, chunks[0]);

    // 底部帮助
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓ 导航", Style::default().fg(Color::DarkGray)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("Enter 查看详情", Style::default().fg(Color::DarkGray)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled("q/Esc 返回", Style::default().fg(Color::DarkGray)),
    ]));

    frame.render_widget(help, chunks[1]);
}
