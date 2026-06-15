//! 无账户时的欢迎界面。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

/// 渲染欢迎界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("欢迎使用 SoloSoul CLI")
            .bold()
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("本地优先 · 零知识 · 你的数据你做主").alignment(Alignment::Center),
        Line::from(""),
        Line::from("当前未发现本地账户。CLI 将引导你创建第一个账户。").alignment(Alignment::Center),
        Line::from(""),
        Line::from("按 Enter 开始创建账户，或 /exit 退出。").alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 欢迎 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
