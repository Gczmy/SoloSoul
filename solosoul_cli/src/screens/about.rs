//! /about 信息屏幕。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::system::AboutInfo;

pub fn render(frame: &mut ratatui::Frame, area: Rect, info: &AboutInfo) {
    let lines = vec![
        Line::from(""),
        Line::from(info.app_name.clone())
            .bold()
            .alignment(Alignment::Center),
        Line::from(format!("版本: {}", info.version)).alignment(Alignment::Center),
        Line::from(format!("平台: {} / {}", info.os, info.arch)).alignment(Alignment::Center),
        Line::from(format!("数据目录: {}", info.data_dir)).alignment(Alignment::Center),
        Line::from(format!(
            "进程锁: {}",
            if info.lock_acquired {
                "已持有（GUI 不可用）"
            } else {
                "未独占"
            }
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from("本地优先 · 零知识 · 你的数据你做主")
            .dark_gray()
            .alignment(Alignment::Center),
    ];

    let paragraph = Paragraph::new(Text::from(lines))
        .block(Block::default().title(" /about ").borders(Borders::ALL))
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
