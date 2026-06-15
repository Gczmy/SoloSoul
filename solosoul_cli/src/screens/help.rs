//! /help 帮助屏幕。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::system::{command_usage, HELP_GROUPS};

pub fn render(frame: &mut ratatui::Frame, area: Rect, topic: &Option<String>) {
    let lines = match topic {
        Some(command) => {
            let usage = command_usage(command).unwrap_or("未知命令，使用 /help 查看全部命令。");
            Text::from(vec![
                Line::from(format!("命令: {}", command)).bold(),
                Line::from(""),
                Line::from(usage),
            ])
        }
        None => {
            let mut lines = vec![
                Line::from("SoloSoul CLI 命令列表")
                    .bold()
                    .alignment(Alignment::Center),
                Line::from(""),
                Line::from("使用 /help <命令> 查看具体用法，例如 /help search")
                    .dark_gray()
                    .alignment(Alignment::Center),
                Line::from(""),
            ];
            for (group, commands) in HELP_GROUPS {
                lines.push(Line::from(format!("[{}]", group)).bold());
                for cmd in *commands {
                    lines.push(Line::from(format!("  {}", cmd)));
                }
                lines.push(Line::from(""));
            }
            Text::from(lines)
        }
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(" /help ").borders(Borders::ALL))
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
