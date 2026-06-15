//! 账户统计屏幕。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::SizeReport;

/// 渲染账户统计。
pub fn render(frame: &mut ratatui::Frame, area: Rect, report: &SizeReport) {
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
        Line::from(format!("页面数量: {}", report.page_count)),
        Line::from(format!("对象数量: {}", report.object_count)),
        Line::from(format!("回收站项目: {}", report.trash_count)),
        Line::from(format!("Profile 数量: {}", report.profile_count)),
        Line::from(format!(
            "Vault 总大小: {}",
            format_bytes(report.total_size_bytes)
        )),
        Line::from(""),
        Line::from("按 Esc 或输入 /back 返回")
            .dark_gray()
            .alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 账户统计 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
