//! 模板列表屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, HighlightSpacing, Paragraph, Row, Table};
use solosoul_core::template_service::SystemTemplate;
use solosoul_core::UserTemplate;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    user_templates: &[UserTemplate],
    system_templates: &[SystemTemplate],
    selected: usize,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from("模板库").bold()).alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let total = user_templates.len() + system_templates.len();
    if total == 0 {
        let hint = Paragraph::new("暂无模板。").alignment(Alignment::Center);
        frame.render_widget(hint, layout[1]);
    } else {
        let header = Row::new(vec!["来源", "名称", "ID", "字段数"])
            .style(Style::default().bold())
            .bottom_margin(1);
        let mut rows: Vec<Row> = Vec::new();
        for (idx, t) in user_templates.iter().enumerate() {
            let style = if idx == selected {
                Style::default().reversed()
            } else {
                Style::default()
            };
            rows.push(
                Row::new(vec![
                    "用户".to_string(),
                    t.name.clone(),
                    t.id.clone(),
                    t.properties.len().to_string(),
                ])
                .style(style),
            );
        }
        for (idx, t) in system_templates.iter().enumerate() {
            let global_idx = user_templates.len() + idx;
            let style = if global_idx == selected {
                Style::default().reversed()
            } else {
                Style::default()
            };
            rows.push(
                Row::new(vec![
                    "系统".to_string(),
                    t.name_fallback.clone(),
                    t.key.clone(),
                    t.properties.len().to_string(),
                ])
                .style(style),
            );
        }
        let mut state = ratatui::widgets::TableState::default().with_selected(Some(selected));
        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Percentage(35),
                Constraint::Percentage(35),
                Constraint::Length(8),
            ],
        )
        .header(header)
        .highlight_spacing(HighlightSpacing::Always)
        .block(Block::default().title(" /template ").borders(Borders::ALL));
        frame.render_stateful_widget(table, layout[1], &mut state);
    }

    frame.render_widget(
        Paragraph::new(
            Line::from("↑↓ 移动 · Enter 查看详情 · D 删除用户模板 · Esc 返回").dark_gray(),
        )
        .alignment(Alignment::Center),
        layout[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_template_list_smoke() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render(frame, frame.area(), &[], &[], 0))
            .unwrap();
        let buf = terminal.backend().buffer();
        let content: String = buf.content.iter().map(|cell| cell.symbol()).collect();
        assert!(content.replace(' ', "").contains("模板库"));
    }
}
