//! 创建对象向导屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};

use crate::app::NewObjectStep;
use crate::i18n::I18n;
use crate::t;
use crate::widgets::field_editor;

pub fn render(frame: &mut ratatui::Frame, area: Rect, step: &NewObjectStep, i18n: &I18n) {
    match step {
        NewObjectStep::SelectPage { pages, selected } => {
            render_select_page(frame, area, pages, *selected, i18n)
        }
        NewObjectStep::SelectTemplate {
            page_name,
            templates,
            selected,
            ..
        } => render_select_template(frame, area, page_name, templates, *selected, i18n),
        NewObjectStep::FillFields {
            page_name,
            name,
            fields,
            selected,
            ..
        } => render_fill_fields(frame, area, page_name, name, fields, *selected, i18n),
    }
}

fn render_select_page(
    frame: &mut ratatui::Frame,
    area: Rect,
    pages: &[solosoul_core::ObjectSummary],
    selected: usize,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from(t!(i18n, "newobj-select-page")).bold(),
        Line::from(t!(i18n, "newobj-select-page-desc")).dark_gray(),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" /newobject {}", t!(i18n, "newobj-create-object"))),
    )
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    if pages.is_empty() {
        let hint = Paragraph::new(t!(i18n, "newobj-no-pages")).alignment(Alignment::Center);
        frame.render_widget(hint, layout[1]);
    } else {
        let rows: Vec<Row> = pages
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let marker = if i == selected { "▸ " } else { "  " };
                let cells = vec![
                    format!("{}{}", marker, p.name),
                    p.id.clone(),
                    p.section_type.clone(),
                ];
                if i == selected {
                    Row::new(cells).style(Style::default().reversed())
                } else {
                    Row::new(cells)
                }
            })
            .collect();
        let header = Row::new(vec!["页面", "ID", "类型"]).style(Style::default().bold());
        let table = Table::new(
            rows,
            [
                Constraint::Percentage(40),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(table, layout[1]);
    }

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "hint-up-down-enter-q")).dark_gray())
            .alignment(Alignment::Center),
        layout[2],
    );
}

fn render_select_template(
    frame: &mut ratatui::Frame,
    area: Rect,
    page_name: &str,
    templates: &[solosoul_core::UserTemplate],
    selected: usize,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from(format!(
            "{} {}）",
            t!(i18n, "newobj-select-template"),
            page_name
        ))
        .bold(),
        Line::from(t!(i18n, "newobj-select-template-desc")).dark_gray(),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" /newobject {}", t!(i18n, "newobj-create-object"))),
    )
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    let mut options: Vec<String> = vec![t!(i18n, "newobj-blank-object").to_string()];
    options.extend(templates.iter().map(|t| t.name.clone()));

    let rows: Vec<Row> = options
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if i == selected { "▸ " } else { "  " };
            let cells = vec![format!("{}{}", marker, name)];
            if i == selected {
                Row::new(cells).style(Style::default().reversed())
            } else {
                Row::new(cells)
            }
        })
        .collect();
    let table = Table::new(rows, [Constraint::Percentage(100)])
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(table, layout[1]);

    frame.render_widget(
        Paragraph::new(Line::from(t!(i18n, "hint-up-down-enter-q")).dark_gray())
            .alignment(Alignment::Center),
        layout[2],
    );
}

fn render_fill_fields(
    frame: &mut ratatui::Frame,
    area: Rect,
    page_name: &str,
    name: &str,
    fields: &[crate::widgets::field_editor::EditableField],
    selected: usize,
    i18n: &I18n,
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Text::from(vec![
        Line::from(format!("创建对象：{}（页面: {}）", name, page_name)).bold(),
        Line::from(t!(i18n, "newobj-fill-fields-hint")).dark_gray(),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" /newobject {}", t!(i18n, "newobj-create-object"))),
    )
    .alignment(Alignment::Center);
    frame.render_widget(title, layout[0]);

    field_editor::render(
        frame,
        layout[1],
        &t!(i18n, "newobj-fields"),
        fields,
        selected,
    );

    let hint = Paragraph::new(Text::from(
        Line::from(t!(i18n, "newobj-fields-nav")).dark_gray(),
    ));
    frame.render_widget(hint, layout[2]);
}
