//! 已锁定状态主界面。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

/// 渲染已锁定主界面。
pub fn render(frame: &mut ratatui::Frame, area: Rect) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from("SoloSoul CLI")
            .bold()
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("当前未登录。可用命令：").alignment(Alignment::Center),
        Line::from(""),
        Line::from("  /unlock        登录账户").alignment(Alignment::Center),
        Line::from("  /account_list  列出本地账户").alignment(Alignment::Center),
        Line::from("  /doctor        诊断数据目录与健康状态").alignment(Alignment::Center),
        Line::from("  /exit          退出").alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 已锁定 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
