//! 字段表单组件（用于对象创建/编辑向导）。

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use solosoul_core::{PropertyType, UserTemplate};

/// 可编辑字段。
#[derive(Debug, Clone)]
pub struct EditableField {
    pub key: String,
    pub label: String,
    pub prop_type: PropertyType,
    pub sensitivity: String,
    pub value: serde_json::Value,
    pub options: Vec<String>,
}

impl EditableField {
    /// 根据对象属性与可选模板构建字段列表。
    pub fn from_properties_and_template(
        properties: &serde_json::Value,
        template: Option<&UserTemplate>,
    ) -> Vec<Self> {
        let mut fields = Vec::new();

        if let Some(tpl) = template {
            for prop in &tpl.properties {
                let value = properties
                    .get(&prop.id)
                    .cloned()
                    .unwrap_or_else(|| default_value(&prop.prop_type));
                fields.push(Self {
                    key: prop.id.clone(),
                    label: prop.name.clone(),
                    prop_type: prop.prop_type.clone(),
                    sensitivity: prop
                        .sensitivity_level
                        .clone()
                        .unwrap_or_else(|| "internal".to_string()),
                    value,
                    options: prop.options.clone().unwrap_or_default(),
                });
            }
        }

        // 如果模板未覆盖某些属性，则以推断类型追加。
        if let serde_json::Value::Object(map) = properties {
            for (k, v) in map {
                if fields.iter().any(|f| f.key == *k) {
                    continue;
                }
                let prop_type = PropertyType::infer_from_value(v, k);
                fields.push(Self {
                    key: k.clone(),
                    label: k.clone(),
                    prop_type,
                    sensitivity: "internal".to_string(),
                    value: v.clone(),
                    options: Vec::new(),
                });
            }
        }

        fields
    }

    pub fn display_value(&self) -> String {
        format_value(&self.value)
    }

    pub fn is_sensitive(&self) -> bool {
        matches!(
            self.sensitivity.to_lowercase().as_str(),
            "sensitive" | "critical" | "restricted"
        )
    }

    /// 编辑前是否需要主密码验证（sensitive / critical / restricted）。
    pub fn requires_password_verification(&self) -> bool {
        self.is_sensitive()
    }

    /// 是否为 critical 级别（需要更严格审计日志）。
    pub fn is_critical(&self) -> bool {
        self.sensitivity.to_lowercase() == "critical"
    }
}

fn default_value(prop_type: &PropertyType) -> serde_json::Value {
    match prop_type {
        PropertyType::Boolean => serde_json::Value::Bool(false),
        PropertyType::Number => serde_json::Value::Number(0.into()),
        PropertyType::MultiSelect => serde_json::Value::Array(vec![]),
        PropertyType::DynamicGroup => serde_json::Value::Array(vec![]),
        _ => serde_json::Value::String(String::new()),
    }
}

pub fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "是" } else { "否" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::Array(arr) => {
            // 动态字段组：展示每个子字段的 name/type/value
            if is_dynamic_group_array(value) {
                return format_dynamic_group(arr);
            }
            arr.iter().map(format_value).collect::<Vec<_>>().join(", ")
        }
        serde_json::Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// 判断一个 JSON 数组是否是动态字段组的子字段列表。
fn is_dynamic_group_array(value: &serde_json::Value) -> bool {
    let Some(arr) = value.as_array() else {
        return false;
    };
    if arr.is_empty() {
        return false;
    }
    arr.iter().all(|item| {
        item.as_object().is_some_and(|o| {
            o.contains_key("id") && o.contains_key("name") && o.contains_key("type")
        })
    })
}

fn format_dynamic_group(arr: &[serde_json::Value]) -> String {
    let parts: Vec<String> = arr
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let name = obj.get("name")?.as_str()?;
            let typ = obj.get("type")?.as_str()?;
            let value = format_value(obj.get("value").unwrap_or(&serde_json::Value::Null));
            Some(format!("{}({}): {}", name, typ, value))
        })
        .collect();
    if parts.is_empty() {
        "(空)".to_string()
    } else {
        parts.join("; ")
    }
}

/// 判断字段是否需要临时退出 ratatui 全屏，通过 inquire 编辑。
pub fn needs_external_editor(field: &EditableField) -> bool {
    matches!(
        field.prop_type,
        PropertyType::Date
            | PropertyType::DateTime
            | PropertyType::Select
            | PropertyType::MultiSelect
            | PropertyType::MultilineText
            | PropertyType::DynamicGroup
    )
}

pub fn mask_value(value: &str) -> String {
    if value.is_empty() {
        value.to_string()
    } else {
        "••••••".to_string()
    }
}

/// 渲染字段列表。`selected` 为当前选中索引。
pub fn render(
    frame: &mut ratatui::Frame,
    area: Rect,
    title: &str,
    fields: &[EditableField],
    selected: usize,
) {
    let header = Row::new(vec!["字段", "类型", "值"])
        .style(Style::default().bold())
        .bottom_margin(1);

    let rows: Vec<Row> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let value_str = if f.is_sensitive() {
                mask_value(&f.display_value())
            } else {
                f.display_value()
            };
            let cells = vec![
                format!("{}{}", if i == selected { "▸ " } else { "  " }, f.label),
                f.prop_type.as_str().to_string(),
                value_str,
            ];
            if i == selected {
                Row::new(cells).style(Style::default().reversed())
            } else {
                Row::new(cells)
            }
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35),
            Constraint::Percentage(20),
            Constraint::Percentage(45),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL),
    );

    frame.render_widget(table, area);
}

/// 渲染字段编辑器底部提示。
pub fn render_hint(frame: &mut ratatui::Frame, area: Rect, editing_name: bool) {
    let hint = if editing_name {
        "↑/↓ 选择字段 · Enter 编辑字段 · n 修改名称 · s 保存 · q 取消"
    } else {
        "↑/↓ 选择字段 · Enter 编辑字段 · s 保存 · q 取消"
    };
    let para = Paragraph::new(Text::from(Line::from(hint).dark_gray()));
    frame.render_widget(para, area);
}

/// 尝试将用户输入的字符串按字段类型解析为 JSON 值。
pub fn parse_value(input: &str, field: &EditableField) -> Result<serde_json::Value, String> {
    match field.prop_type {
        PropertyType::Boolean => {
            let lower = input.trim().to_lowercase();
            if lower == "true" || lower == "是" || lower == "1" {
                Ok(serde_json::Value::Bool(true))
            } else if lower == "false" || lower == "否" || lower == "0" {
                Ok(serde_json::Value::Bool(false))
            } else {
                Err("请输入 true/false 或 是/否".to_string())
            }
        }
        PropertyType::Number => input
            .trim()
            .parse::<serde_json::Number>()
            .map(serde_json::Value::Number)
            .map_err(|_| "请输入数字".to_string()),
        PropertyType::MultiSelect => {
            let parts: Vec<serde_json::Value> = input
                .split(',')
                .map(|s| serde_json::Value::String(s.trim().to_string()))
                .filter(|v| !v.as_str().unwrap_or("").is_empty())
                .collect();
            Ok(serde_json::Value::Array(parts))
        }
        PropertyType::DynamicGroup => {
            // 动态字段组通过外部编辑器处理，内部输入框仅做占位
            Ok(serde_json::Value::Array(vec![]))
        }
        _ => Ok(serde_json::Value::String(input.to_string())),
    }
}

/// 为指定字段生成合适的提示规格。
pub fn prompt_for_field(field: &EditableField) -> crate::widgets::prompt::PromptSpec {
    use crate::widgets::prompt::PromptSpec;

    let initial = field.display_value();
    match field.prop_type {
        PropertyType::Boolean => PromptSpec::Confirm {
            message: format!("{}: 设置为真？", field.label),
            default_yes: field.value.as_bool().unwrap_or(false),
        },
        PropertyType::Select if !field.options.is_empty() => {
            let selected = field
                .options
                .iter()
                .position(|o| o == &initial)
                .unwrap_or(0);
            PromptSpec::Select {
                label: field.label.clone(),
                options: field.options.clone(),
                selected,
            }
        }
        _ => PromptSpec::Text {
            label: field.label.clone(),
            initial,
            mask: field.is_sensitive(),
            allow_toggle_mask: field.is_sensitive(),
        },
    }
}

/// 从 PromptResult 中提取字段新值。
pub fn value_from_result(
    result: &crate::widgets::prompt::PromptResult,
    field: &EditableField,
) -> Option<serde_json::Value> {
    use crate::widgets::prompt::PromptResult;
    match result {
        PromptResult::Text(s) => parse_value(s, field).ok(),
        PromptResult::Select(idx) => {
            if field.prop_type == PropertyType::Select && !field.options.is_empty() {
                field
                    .options
                    .get(*idx)
                    .cloned()
                    .map(serde_json::Value::String)
            } else if field.prop_type == PropertyType::Boolean {
                Some(serde_json::Value::Bool(*idx == 0))
            } else {
                None
            }
        }
        PromptResult::Confirm(b) => Some(serde_json::Value::Bool(*b)),
        PromptResult::Cancel => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_dynamic_group_value() {
        let value = serde_json::json!([
            { "id": "1", "name": "手机", "type": "phone", "value": "13800138000" },
            { "id": "2", "name": "邮箱", "type": "email", "value": "a@b.com" }
        ]);
        let s = format_value(&value);
        assert!(s.contains("手机(phone): 13800138000"));
        assert!(s.contains("邮箱(email): a@b.com"));
    }

    #[test]
    fn needs_external_editor_includes_dynamic_group() {
        let field = EditableField {
            key: "contactMethods".to_string(),
            label: "联系方式".to_string(),
            prop_type: PropertyType::DynamicGroup,
            sensitivity: "internal".to_string(),
            value: serde_json::Value::Array(vec![]),
            options: vec![],
        };
        assert!(needs_external_editor(&field));
    }

    #[test]
    fn default_value_for_dynamic_group_is_empty_array() {
        assert_eq!(
            default_value(&PropertyType::DynamicGroup),
            serde_json::Value::Array(vec![])
        );
    }
}
