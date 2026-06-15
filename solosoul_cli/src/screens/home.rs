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
        Line::from("  /list [页面名]       列出页面或页面内对象").alignment(Alignment::Center),
        Line::from("  /open <对象ID>       查看对象详情").alignment(Alignment::Center),
        Line::from("  /size                账户统计").alignment(Alignment::Center),
        Line::from("  /search <关键词>     全局搜索").alignment(Alignment::Center),
        Line::from("  /history <对象ID>    历史快照").alignment(Alignment::Center),
        Line::from("  /newpage <名称>      创建页面").alignment(Alignment::Center),
        Line::from("  /newobject [页面]    创建对象").alignment(Alignment::Center),
        Line::from("  /edit <对象ID>       编辑对象").alignment(Alignment::Center),
        Line::from("  /delete <对象ID>     删除对象/页面").alignment(Alignment::Center),
        Line::from("  /trash               回收站").alignment(Alignment::Center),
        Line::from("  /restore <id>        恢复回收站项目").alignment(Alignment::Center),
        Line::from("  /purge <id>          彻底删除").alignment(Alignment::Center),
        Line::from("  /operation_log [N]   审计日志").alignment(Alignment::Center),
        Line::from("  /export_log [文件名] 导出审计日志").alignment(Alignment::Center),
        Line::from("  /about               关于").alignment(Alignment::Center),
        Line::from("  /help [命令]         帮助").alignment(Alignment::Center),
        Line::from(""),
        Line::from("会话：").alignment(Alignment::Center),
        Line::from("  /lock 或 /logout     锁定 Vault").alignment(Alignment::Center),
        Line::from("  /exit                安全退出").alignment(Alignment::Center),
    ]);

    let paragraph = Paragraph::new(text)
        .block(Block::default().title(" 首页 ").borders(Borders::ALL))
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
