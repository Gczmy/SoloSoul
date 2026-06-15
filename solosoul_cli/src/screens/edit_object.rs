//! 编辑对象向导屏幕。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use solosoul_core::ObjectRecord;

use crate::app::EditObjectStep;
use crate::widgets::field_editor;

pub fn render(frame: &mut ratatui::Frame, area: Rect, _object_id: &str, step: &EditObjectStep) {
    match step {
        EditObjectStep::Overview {
            object,
            fields,
            selected,
        } => render_overview(frame, area, object, fields, *selected),
    }
}

fn render_overview(
    frame: &mut ratatui::Frame,
    area: Rect,
    object: &ObjectRecord,
    fields: &[crate::widgets::field_editor::EditableField],
    selected: usize,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let meta = Paragraph::new(Text::from(vec![
        Line::from(format!("名称: {}", object.name)).bold(),
        Line::from(format!("ID: {}", object.id)).dark_gray(),
        Line::from(format!("类型: {}", object.type_id)),
        Line::from(format!(
            "模板: {}",
            object.template_type.as_deref().unwrap_or("无")
        )),
        Line::from(format!("敏感度: {}", object.sensitivity_level)),
    ]))
    .block(Block::default().title(" 对象信息 ").borders(Borders::ALL));
    frame.render_widget(meta, layout[0]);

    field_editor::render(frame, layout[1], "属性", fields, selected);

    field_editor::render_hint(frame, layout[2], true);
}
