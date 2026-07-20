//! /ocr 结果屏：显示 OCR 文本块、档位状态或 MRZ 结构化字段。

use crate::i18n::I18n;
use crate::t;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use solosoul_core::ocr::types::{MrzResult, OcrResult};

#[derive(Debug, Clone)]
pub struct TierEntry {
    pub name: String,
    pub installed: bool,
    pub size_mb: f32,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    result: &OcrResult,
    source_path: &str,
    tiers: &[TierEntry],
    mrz: Option<&MrzResult>,
    i18n: &I18n,
) {
    if let Some(mrz_data) = mrz {
        render_mrz(frame, area, mrz_data, source_path, i18n);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

    let header_text = if source_path.is_empty() {
        "OCR".to_string()
    } else {
        format!("OCR · {}", source_path)
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            header_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                "blocks={} conf={:.2}",
                result.boxes.len(),
                result.confidence
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    if !tiers.is_empty() {
        let items: Vec<ListItem> = tiers
            .iter()
            .map(|t| {
                let status = if t.installed {
                    Span::styled("[installed] ", Style::default().fg(Color::Green))
                } else {
                    Span::styled("[missing] ", Style::default().fg(Color::Red))
                };
                let line = Line::from(vec![
                    status,
                    Span::styled(&t.name, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(
                        format!("~{:.0} MB", t.size_mb),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(t!(i18n, "ocr-tiers")),
        );
        frame.render_widget(list, chunks[1]);
    } else {
        // OCR 文本块视图
        let body = if result.boxes.is_empty() && result.text.is_empty() {
            Paragraph::new(Line::from(t!(i18n, "ocr-no-text")))
                .style(Style::default().fg(Color::DarkGray))
        } else {
            let lines: Vec<Line> = if result.boxes.is_empty() {
                result
                    .text
                    .lines()
                    .map(|l| Line::from(l.to_string()))
                    .collect()
            } else {
                result
                    .boxes
                    .iter()
                    .map(|b| {
                        Line::from(vec![
                            Span::styled(
                                format!("[{:.2}] ", b.confidence),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::raw(&b.text),
                        ])
                    })
                    .collect()
            };
            Paragraph::new(lines).wrap(Wrap { trim: false })
        };
        frame.render_widget(
            body.block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(t!(i18n, "ocr-recognized-text")),
            ),
            chunks[1],
        );
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            t!(i18n, "ocr-hint-prefix"),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(": "),
        Span::raw(t!(i18n, "ocr-hint-text")),
    ]));
    frame.render_widget(hint, chunks[2]);
}

/// 渲染 MRZ 结构化字段（护照/旅行文件）。
fn render_mrz(frame: &mut Frame, area: Rect, m: &MrzResult, source: &str, i18n: &I18n) {
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
            format!("MRZ · {}", source),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!(
                "conf={:.2} checksum={}",
                m.confidence,
                if m.checksum_valid { "✓" } else { "✗" }
            ),
            Style::default().fg(if m.checksum_valid {
                Color::Green
            } else {
                Color::Red
            }),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, chunks[0]);

    let rows: Vec<(String, String)> = vec![
        (
            "证件类型".to_string(),
            format!(
                "{}{}",
                m.document_type,
                if m.document_type_sub.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", m.document_type_sub)
                }
            ),
        ),
        ("签发国".to_string(), m.issuing_country.clone()),
        ("证件号".to_string(), m.document_number.clone()),
        (
            "校验位(号)".to_string(),
            m.check_digit_document_number.to_string(),
        ),
        ("国籍".to_string(), m.nationality.clone()),
        ("出生日期".to_string(), m.date_of_birth.clone()),
        (
            "校验位(出生)".to_string(),
            m.check_digit_date_of_birth.to_string(),
        ),
        ("性别".to_string(), m.sex.clone()),
        ("有效期".to_string(), m.expiry_date.clone()),
        ("校验位(有效)".to_string(), m.check_digit_expiry.to_string()),
        ("综合校验位".to_string(), m.composite_check_digit.clone()),
        ("可选数据".to_string(), m.optional_data.clone()),
        ("原始行".to_string(), m.raw_lines.join(" | ")),
    ];

    let items: Vec<ListItem> = rows
        .into_iter()
        .map(|(k, v)| {
            // 转移 v 为 owned String,避免 Span<'_> 借用已 drop 的元组字段。
            let value_display: String = if v.is_empty() { "—".to_string() } else { v };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<14}", k), Style::default().fg(Color::Cyan)),
                Span::raw(value_display),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(t!(i18n, "ocr-mrz-fields")),
    );
    frame.render_widget(list, chunks[1]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            t!(i18n, "ocr-hint-prefix"),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(": "),
        Span::raw(t!(i18n, "ocr-mrz-hint-text")),
    ]));
    frame.render_widget(hint, chunks[2]);
}
