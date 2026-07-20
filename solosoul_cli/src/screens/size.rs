//! 账户统计屏幕。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::SizeReport;
use crate::i18n::I18n;
use crate::t;

/// 渲染账户统计。
pub fn render(frame: &mut ratatui::Frame, area: Rect, report: &SizeReport, i18n: &I18n) {
    fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        if bytes == 0 {
            return "0 B".to_string();
        }
        let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
        let value = bytes as f64 / 1024f64.powi(exp as i32);
        format!("{:.2} {}", value, UNITS[exp])
    }

    let text = Text::from(vec![
        Line::from(""),
        Line::from(t!(
            i18n,
            "size-pages",
            count = report.page_count.to_string()
        )),
        Line::from(t!(
            i18n,
            "size-objects",
            count = report.object_count.to_string()
        )),
        Line::from(t!(
            i18n,
            "size-trash",
            count = report.trash_count.to_string()
        )),
        Line::from(t!(
            i18n,
            "size-profiles",
            count = report.profile_count.to_string()
        )),
        Line::from(t!(
            i18n,
            "size-total-size",
            size = format_bytes(report.total_size_bytes)
        )),
        Line::from(""),
        Line::from(t!(i18n, "hint-esc-or-back"))
            .dark_gray()
            .alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 账户统计 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
