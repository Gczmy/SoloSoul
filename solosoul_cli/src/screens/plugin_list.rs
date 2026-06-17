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
    filter: &str,
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
        format!("插件列表 (共 {} 个)", plugins.len())
    } else {
        format!(
            "插件列表: \"{}\" ({} / {} 个)",
            filter,
            filtered.len(),
            plugins.len()
        )
    };

    let items: Vec<ListItem> = if filtered.is_empty() {
        let msg = if filter.is_empty() {
            "暂无可用插件"
        } else {
            "无匹配结果"
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
        "↑↓ 导航  键入过滤  |  Enter 详情  r 运行  i 安装  u 更新  d 卸载  |  q/Esc 返回"
    } else {
        "输入关键字过滤  Esc 清除  Backspace 删除  |  ↑↓ 导航  Enter 详情  r 运行"
    };
    let help = Paragraph::new(Line::from(vec![Span::styled(
        hint_text,
        Style::default().fg(Color::DarkGray),
    )]));

    frame.render_widget(help, chunks[1]);
}
