//! /doctor 结果展示界面。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::doctor::DoctorReport;

/// 渲染 doctor 诊断结果。
pub fn render(frame: &mut ratatui::Frame, area: Rect, report: &DoctorReport) {
    let mut lines = vec![
        Line::from(""),
        Line::from("诊断报告").bold().alignment(Alignment::Center),
        Line::from(""),
    ];

    lines.push(Line::from(format!("数据目录: {}", report.data_dir)));
    lines.push(Line::from(format!(
        "数据目录状态: {}",
        if report.data_dir_exists {
            "存在"
        } else {
            "不存在"
        }
    )));
    lines.push(Line::from(format!(
        "数据目录可写: {}",
        if report.data_dir_writable {
            "是"
        } else {
            "否"
        }
    )));
    lines.push(Line::from(format!("账户数量: {}", report.account_count)));
    if !report.account_errors.is_empty() {
        lines.push(Line::from("账户异常:").yellow());
        for err in &report.account_errors {
            lines.push(Line::from(format!("  - {}", err)).yellow());
        }
    }
    lines.push(Line::from(format!("核心库版本: {}", report.core_version)));
    lines.push(Line::from(format!("Vault 版本: {}", report.vault_version)));
    lines.push(Line::from(format!("平台: {}", report.platform)));
    lines.push(Line::from(format!(
        "进程锁状态: {}",
        if report.lock_acquired {
            "已获取（无其他实例运行）"
        } else {
            "未获取（可能有其他实例运行）"
        }
    )));
    lines.push(Line::from(format!("日志路径: {}", report.log_path)));

    lines.push(Line::from(""));
    lines.push(
        Line::from("按 Esc 或输入 /back 返回")
            .dark_gray()
            .alignment(Alignment::Center),
    );

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" /doctor ").borders(Borders::ALL))
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
