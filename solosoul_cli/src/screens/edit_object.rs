//! 编辑对象向导屏幕。

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use solosoul_core::ObjectRecord;

use crate::app::EditObjectStep;
use crate::i18n::I18n;
use crate::t;
use crate::widgets::field_editor;

pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    _object_id: &str,
    step: &EditObjectStep,
    i18n: &I18n,
) {
    match step {
        EditObjectStep::Overview {
            object,
            fields,
            selected,
        } => render_overview(frame, area, object, fields, *selected, i18n),
    }
}

fn render_overview(
    frame: &mut ratatui::Frame,
    area: Rect,
    object: &ObjectRecord,
    fields: &[crate::widgets::field_editor::EditableField],
    selected: usize,
    i18n: &I18n,
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
        Line::from(t!(i18n, "object-detail-name", name = &object.name)).bold(),
        Line::from(t!(i18n, "object-detail-id", id = &object.id)).dark_gray(),
        Line::from(t!(i18n, "object-detail-type", r#type = &object.type_id)),
        Line::from(t!(
            i18n,
            "editobj-template",
            tpl = object.template_type.as_deref().unwrap_or("")
        )),
        Line::from(t!(
            i18n,
            "object-detail-sensitivity",
            level = &object.sensitivity_level
        )),
    ]))
    .block(
        Block::default()
            .title(format!(" {} ", t!(i18n, "editobj-object-info")))
            .borders(Borders::ALL),
    );
    frame.render_widget(meta, layout[0]);

    field_editor::render(
        frame,
        layout[1],
        &t!(i18n, "editobj-properties"),
        fields,
        selected,
    );

    field_editor::render_hint(frame, layout[2], true);
}
