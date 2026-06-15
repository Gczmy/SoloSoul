//! 已登录首页。

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

/// 渲染已登录首页。
pub fn render(frame: &mut ratatui::Frame, area: Rect, account_id: &str) {
    let text = Text::from(vec![
        Line::from(""),
        Line::from(format!("欢迎回来 · {}", account_id))
            .bold()
            .alignment(Alignment::Center),
        Line::from(""),
        Line::from("数据操作：").alignment(Alignment::Center),
        Line::from("  /list [页面名]    列出页面或页面内对象").alignment(Alignment::Center),
        Line::from("  /open <对象ID>    查看对象详情").alignment(Alignment::Center),
        Line::from("  /size             账户统计").alignment(Alignment::Center),
        Line::from(""),
        Line::from("会话：").alignment(Alignment::Center),
        Line::from("  /lock 或 /logout  锁定 Vault").alignment(Alignment::Center),
        Line::from("  /exit             安全退出").alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 首页 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
