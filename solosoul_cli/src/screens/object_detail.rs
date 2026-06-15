//! 对象详情屏幕。

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::ObjectRecord;

/// 渲染对象详情。
pub fn render(frame: &mut ratatui::Frame, area: Rect, object: &ObjectRecord) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    // 元信息
    let meta_text = Text::from(vec![
        Line::from(format!("名称: {}", object.name)).bold(),
        Line::from(format!("ID: {}", object.id)).dark_gray(),
        Line::from(format!("类型: {}", object.type_id)),
        Line::from(format!(
            "模板: {}",
            object.template_type.as_deref().unwrap_or("无")
        )),
        Line::from(format!("敏感度: {}", object.sensitivity_level)),
        Line::from(format!("版本: {}", object.version)),
    ]);
    let meta =
        Paragraph::new(meta_text).block(Block::default().title(" 对象信息 ").borders(Borders::ALL));
    frame.render_widget(meta, layout[0]);

    // 属性列表
    let should_mask = should_mask_level(&object.sensitivity_level);
    let header = Row::new(vec!["字段", "值"])
        .style(ratatui::style::Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = if let serde_json::Value::Object(map) = &object.properties {
        map.iter()
            .map(|(k, v)| {
                let value_str = format_value(v);
                let display = if should_mask {
                    mask(&value_str)
                } else {
                    value_str
                };
                Row::new(vec![k.clone(), display])
            })
            .collect()
    } else {
        vec![Row::new(vec![
            "properties".to_string(),
            format_value(&object.properties),
        ])]
    };

    let table = Table::new(
        rows,
        [Constraint::Percentage(35), Constraint::Percentage(65)],
    )
    .header(header)
    .block(Block::default().title(" 属性 ").borders(Borders::ALL));
    frame.render_widget(table, layout[1]);

    // 底部提示
    if should_mask {
        let hint = Paragraph::new(
            Line::from("敏感对象：属性值已掩码。编辑模式下可验证主密码后查看。").dark_gray(),
        )
        .alignment(Alignment::Center);
        let hint_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        frame.render_widget(hint, hint_area);
    }
}

fn should_mask_level(level: &str) -> bool {
    matches!(
        level.to_lowercase().as_str(),
        "sensitive" | "critical" | "restricted"
    )
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "是" } else { "否" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::Array(arr) => {
            arr.iter().map(format_value).collect::<Vec<_>>().join(", ")
        }
        serde_json::Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn mask(value: &str) -> String {
    if value.is_empty() {
        return value.to_string();
    }
    "••••••".to_string()
}
