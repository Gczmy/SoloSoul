//! /doctor 结果展示界面。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::doctor::DoctorReport;
use crate::i18n::I18n;
use crate::t;

/// 渲染 doctor 诊断结果。
pub fn render(frame: &mut ratatui::Frame, area: Rect, report: &DoctorReport, i18n: &I18n) {
    let mut lines = vec![
        Line::from(""),
        Line::from(t!(i18n, "doctor-title"))
            .bold()
            .alignment(Alignment::Center),
        Line::from(""),
    ];

    lines.push(Line::from(t!(
        i18n,
        "doctor-data-dir",
        path = report.data_dir
    )));
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
    lines.push(Line::from(t!(
        i18n,
        "doctor-account-count",
        count = report.account_count.to_string()
    )));
    if !report.account_errors.is_empty() {
        lines.push(Line::from(t!(i18n, "doctor-account-issues")).yellow());
        for err in &report.account_errors {
            lines.push(Line::from(format!("  - {}", err)).yellow());
        }
    }
    lines.push(Line::from(t!(
        i18n,
        "doctor-core-version",
        ver = report.core_version
    )));
    lines.push(Line::from(t!(
        i18n,
        "doctor-vault-version",
        ver = report.vault_version
    )));
    lines.push(Line::from(t!(
        i18n,
        "doctor-platform",
        os = report.platform,
        arch = ""
    )));
    lines.push(Line::from(format!(
        "进程锁状态: {}",
        if report.lock_acquired {
            "已获取（无其他实例运行）"
        } else {
            "未获取（可能有其他实例运行）"
        }
    )));
    lines.push(Line::from(t!(
        i18n,
        "doctor-log-path",
        path = report.log_path
    )));

    lines.push(Line::from(""));
    lines.push(
        Line::from(t!(i18n, "hint-esc-or-back"))
            .dark_gray()
            .alignment(Alignment::Center),
    );

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!(" /doctor {}", t!(i18n, "doctor-title")))
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    frame.render_widget(paragraph, area);
}
