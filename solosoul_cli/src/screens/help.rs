//! /help 帮助屏幕。
//!
//! 无参数时显示分组命令列表，每类带边框、80% 宽度居中、命令+描述左对齐。
//! 超出终端高度时支持滚动。参数 /help <命令> 显示单个命令用法。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::commands::system::{command_usage, HELP_GROUPS};
use crate::theme::Theme;

/// 渲染帮助屏幕。
///
/// `scroll_offset` 为已滚动行数（仅在无参数时生效）。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    topic: &Option<String>,
    scroll_offset: usize,
) {
    let theme = Theme::load();

    match topic {
        Some(command) => {
            let usage = command_usage(command).unwrap_or("未知命令，使用 /help 查看全部命令。");
            let lines = Text::from(vec![
                Line::from(format!(" 命令: {} ", command)).bold(),
                Line::from(""),
                Line::from(format!(" {}", usage)),
            ]);
            let paragraph = Paragraph::new(lines)
                .block(Block::default().title(" /help ").borders(Borders::ALL))
                .alignment(Alignment::Left);
            frame.render_widget(paragraph, area);
        }
        None => {
            let inner_width = (area.width as usize * 80 / 100).max(40) as u16;
            let left_margin = (area.width.saturating_sub(inner_width)) / 2;
            let content_area = Rect {
                x: area.x + left_margin,
                y: area.y,
                width: inner_width,
                height: area.height,
            };

            // Build all text lines for the grouped help
            let mut all_lines: Vec<Line<'static>> = Vec::new();

            // Header
            all_lines.push(
                Line::from(" Solo S o u l  C L I ")
                    .bold()
                    .alignment(Alignment::Center),
            );
            all_lines.push(
                Line::from(" 使用 /help <命令> 查看具体用法")
                    .style(theme.style_muted())
                    .alignment(Alignment::Center),
            );
            all_lines.push(Line::from(""));

            for (group_name, entries) in HELP_GROUPS {
                // Top border with category title
                let title_prefix = format!("─[{}]─", group_name);
                let title_fill = inner_width.saturating_sub(title_prefix.len() as u16 + 2);
                let top_line = format!("┌{}─{}┐", title_prefix, "─".repeat(title_fill as usize));
                all_lines.push(Line::from(top_line).style(theme.style_brand()));

                // Commands
                for entry in *entries {
                    let cmd_padded = format!("  {} ", entry.command);
                    let desc = entry.description;
                    // Fill remaining space with spaces, leaving room for right border
                    let cmd_len = cmd_padded.chars().count();
                    let max_desc = inner_width as usize - cmd_len - 3;
                    let desc_trunc = if desc.chars().count() > max_desc {
                        format!("{}…", desc.chars().take(max_desc - 1).collect::<String>())
                    } else {
                        desc.to_string()
                    };
                    let line_str = format!(
                        "│{} {} {}",
                        cmd_padded,
                        desc_trunc,
                        " ".repeat(
                            (inner_width as usize)
                                .saturating_sub(cmd_len + desc_trunc.chars().count() + 3)
                        )
                    );
                    all_lines.push(Line::from(line_str).style(theme.style_text()));
                }

                // Bottom border
                let bottom_line = "└".to_string() + &"─".repeat(inner_width as usize - 2) + "┘";
                all_lines.push(Line::from(bottom_line).style(theme.style_brand()));
                all_lines.push(Line::from(""));
            }

            let text = Text::from(all_lines);
            let paragraph = Paragraph::new(text)
                .block(Block::default().title(" /help ").borders(Borders::ALL))
                .scroll((scroll_offset as u16, 0));
            frame.render_widget(paragraph, content_area);
        }
    }
}
