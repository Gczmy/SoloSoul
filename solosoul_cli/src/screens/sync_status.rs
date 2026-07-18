//! /sync 状态屏：列出 vault 已持久化的 peers。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use solosoul_sync::types::SyncPeerInfo;

pub fn render(frame: &mut Frame, area: Rect, peers: &[SyncPeerInfo], info: &str) {
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
            "设备同步 (Sync)",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(info, Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    if peers.is_empty() {
        let empty = Paragraph::new(Line::from(vec![
            Span::styled("无持久化 peers。", Style::default().fg(Color::Yellow)),
            Span::raw("\n"),
            Span::raw(
                "提示: 使用 `/sync with <host:port>` 与 GUI 实例同步后，此处会出现 peer 记录。",
            ),
        ]))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Peer 列表"));
        frame.render_widget(empty, chunks[1]);
        let hint = Paragraph::new(Line::from(vec![
            Span::styled("子命令", Style::default().fg(Color::Cyan)),
            Span::raw(": "),
            Span::raw("/sync status | /sync with <addr> | /sync trust <id> | /sync forget <id>"),
        ]));
        frame.render_widget(hint, chunks[2]);
        return;
    }

    let items: Vec<ListItem> = peers
        .iter()
        .map(|p| {
            let trusted = if p.trusted {
                Span::styled("[trusted] ", Style::default().fg(Color::Green))
            } else {
                Span::styled("[untrusted] ", Style::default().fg(Color::Red))
            };
            let line = Line::from(vec![
                trusted,
                Span::styled(&p.node_id, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("  fp="),
                Span::styled(&p.fingerprint, Style::default().fg(Color::Magenta)),
                Span::raw("  name="),
                Span::raw(&p.name),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Peer 列表"));
    frame.render_widget(list, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("提示", Style::default().fg(Color::Cyan)),
        Span::raw(": 要与某 peer 同步，请先 `"),
        Span::styled("/sync trust <id>", Style::default().fg(Color::Yellow)),
        Span::raw("` 再通过 GUI 启用持续同步；CLI 内 `/sync with <host:port>` 是一次性会话。"),
    ]));
    frame.render_widget(hint, chunks[2]);
}

/// 满足 mod.rs 中 crates 需要的占位函数（真实 render 在 render()）。
pub fn render_dummy() {}
