//! 需要临时退出 ratatui 全屏、通过 inquire 编辑的字段处理器。

use std::io::{stdout, Write};

use chrono::TimeZone;
use color_eyre::Result;
use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use inquire::error::InquireError;

use crate::app::ExternalEditRequest;
use crate::widgets::field_editor::EditableField;

const ALL_PROPERTY_TYPES: [&str; 12] = [
    "text",
    "multiline",
    "number",
    "date",
    "datetime",
    "boolean",
    "select",
    "multiselect",
    "url",
    "email",
    "phone",
    "file",
];

/// 在终端普通模式下运行 inquire，返回新的字段值；用户取消返回 `Ok(None)`。
pub fn run(request: &ExternalEditRequest) -> Result<Option<serde_json::Value>> {
    leave_ratatui()?;
    let result = match request {
        ExternalEditRequest::Date(field) => edit_date(field),
        ExternalEditRequest::DateTime(field) => edit_datetime(field),
        ExternalEditRequest::Select(field) => edit_select(field),
        ExternalEditRequest::MultiSelect(field) => edit_multi_select(field),
        ExternalEditRequest::Textarea(field) => edit_textarea(field),
        ExternalEditRequest::DynamicGroup(field) => edit_dynamic_group(field),
    };
    enter_ratatui()?;
    result
}

fn edit_date(field: &EditableField) -> Result<Option<serde_json::Value>> {
    let initial = parse_date(&field.display_value());
    let ans = inquire::DateSelect::new(&field.label)
        .with_default(initial.unwrap_or_else(chrono::Local::now).date_naive())
        .prompt();
    match ans {
        Ok(date) => Ok(Some(serde_json::Value::String(format!("{}", date)))),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(color_eyre::eyre::Report::from(e)),
    }
}

fn edit_datetime(field: &EditableField) -> Result<Option<serde_json::Value>> {
    let initial = parse_date(&field.display_value()).unwrap_or_else(chrono::Local::now);
    let date_ans = inquire::DateSelect::new(&format!("{} · 日期", field.label))
        .with_default(initial.date_naive())
        .prompt();
    let date = match date_ans {
        Ok(d) => d,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            return Ok(None)
        }
        Err(e) => return Err(color_eyre::eyre::Report::from(e)),
    };

    let time_str = inquire::Text::new(&format!("{} · 时间 (HH:MM:SS)", field.label))
        .with_default(&initial.format("%H:%M:%S").to_string())
        .with_help_message("例如 14:30:00")
        .prompt();
    let time_str = match time_str {
        Ok(t) => t,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            return Ok(None)
        }
        Err(e) => return Err(color_eyre::eyre::Report::from(e)),
    };

    let datetime = chrono::NaiveDateTime::parse_from_str(
        &format!("{} {}", date, time_str),
        "%Y-%m-%d %H:%M:%S",
    );
    match datetime {
        Ok(dt) => Ok(Some(serde_json::Value::String(
            chrono::Local
                .from_local_datetime(&dt)
                .single()
                .unwrap_or_else(chrono::Local::now)
                .to_rfc3339(),
        ))),
        Err(_) => {
            // 时间格式错误时返回原值，避免数据丢失
            Ok(Some(field.value.clone()))
        }
    }
}

fn edit_select(field: &EditableField) -> Result<Option<serde_json::Value>> {
    if field.options.is_empty() {
        return Ok(None);
    }
    let initial = field
        .options
        .iter()
        .position(|o| o == &field.display_value());
    let mut select = inquire::Select::new(&field.label, field.options.clone());
    if let Some(idx) = initial {
        select = select.with_starting_filter_input(&field.options[idx]);
    }
    match select.prompt() {
        Ok(value) => Ok(Some(serde_json::Value::String(value))),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(color_eyre::eyre::Report::from(e)),
    }
}

fn edit_multi_select(field: &EditableField) -> Result<Option<serde_json::Value>> {
    if field.options.is_empty() {
        return Ok(None);
    }
    let current: Vec<String> = match &field.value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => vec![],
    };
    let default_indices: Vec<usize> = current
        .iter()
        .filter_map(|c| field.options.iter().position(|o| o == c))
        .collect();
    let ans = inquire::MultiSelect::new(&field.label, field.options.clone())
        .with_default(&default_indices)
        .prompt();
    match ans {
        Ok(values) => Ok(Some(serde_json::Value::Array(
            values.into_iter().map(serde_json::Value::String).collect(),
        ))),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(color_eyre::eyre::Report::from(e)),
    }
}

fn edit_textarea(field: &EditableField) -> Result<Option<serde_json::Value>> {
    let initial = field.display_value();
    let ans = inquire::Editor::new(&field.label)
        .with_predefined_text(&initial)
        .with_file_extension(".md")
        .prompt();
    match ans {
        Ok(value) => Ok(Some(serde_json::Value::String(value))),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(color_eyre::eyre::Report::from(e)),
    }
}

fn parse_date(s: &str) -> Option<chrono::DateTime<chrono::Local>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Local))
        .or_else(|| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| {
                    d.and_hms_opt(0, 0, 0)
                        .and_then(|ndt| chrono::Local.from_local_datetime(&ndt).single())
                })
        })
}

fn edit_dynamic_group(field: &EditableField) -> Result<Option<serde_json::Value>> {
    let mut items = match &field.value {
        serde_json::Value::Array(arr) => arr.clone(),
        _ => vec![],
    };

    loop {
        let choices = build_dynamic_group_choices(&items);
        let ans = inquire::Select::new(&format!("{} · 选择操作", field.label), choices)
            .with_help_message("a=添加 e=编辑 d=删除 ↑/↓=选择 Enter=确认 q/Esc=完成")
            .prompt();

        match ans {
            Ok(choice) => match choice.as_str() {
                "[完成]" => break,
                "[添加子字段]" => {
                    if let Some(new_item) = prompt_dynamic_group_item(None) {
                        items.push(new_item);
                    }
                }
                s if s.starts_with("[删除] ") => {
                    if let Some(idx) = parse_choice_index(s) {
                        if idx < items.len() {
                            items.remove(idx);
                        }
                    }
                }
                s if s.starts_with("[编辑] ") => {
                    if let Some(idx) = parse_choice_index(s) {
                        if idx < items.len() {
                            if let Some(updated) = prompt_dynamic_group_item(items.get(idx)) {
                                items[idx] = updated;
                            }
                        }
                    }
                }
                _ => {}
            },
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                // 取消时保留已修改的内容，视为完成
                break;
            }
            Err(e) => return Err(color_eyre::eyre::Report::from(e)),
        }
    }

    Ok(Some(serde_json::Value::Array(items)))
}

fn build_dynamic_group_choices(items: &[serde_json::Value]) -> Vec<String> {
    let mut choices = vec!["[添加子字段]".to_string(), "[完成]".to_string()];
    for (i, item) in items.iter().enumerate() {
        let label = format_dynamic_item_label(item).unwrap_or_else(|| format!("条目 {}", i + 1));
        choices.push(format!("[编辑] {}. {}", i + 1, label));
        choices.push(format!("[删除] {}. {}", i + 1, label));
    }
    choices
}

fn format_dynamic_item_label(item: &serde_json::Value) -> Option<String> {
    let obj = item.as_object()?;
    let name = obj.get("name")?.as_str()?;
    let typ = obj.get("type")?.as_str()?;
    Some(format!("{} ({})", name, typ))
}

fn parse_choice_index(s: &str) -> Option<usize> {
    s.split('.')
        .next()
        .and_then(|prefix| prefix.split_whitespace().last())
        .and_then(|n| n.parse::<usize>().ok())
        .map(|n| n.saturating_sub(1))
}

fn prompt_dynamic_group_item(existing: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let existing_obj = existing.and_then(|v| v.as_object());
    let existing_name = existing_obj
        .and_then(|o| o.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let existing_type = existing_obj
        .and_then(|o| o.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    let name = match inquire::Text::new("子字段名称")
        .with_default(existing_name)
        .prompt()
    {
        Ok(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => return None,
    };

    let typ = match inquire::Select::new(
        "子字段类型",
        ALL_PROPERTY_TYPES.iter().map(|s| s.to_string()).collect(),
    )
    .with_starting_filter_input(existing_type)
    .prompt()
    {
        Ok(t) => t,
        Err(_) => return None,
    };

    let existing_value = existing_obj
        .and_then(|o| o.get("value"))
        .cloned()
        .unwrap_or_else(|| default_value_for_type(&typ));

    let value = match typ.as_str() {
        "boolean" => {
            let yes = existing_value.as_bool().unwrap_or(false);
            match inquire::Confirm::new("值").with_default(yes).prompt() {
                Ok(b) => serde_json::Value::Bool(b),
                Err(_) => return None,
            }
        }
        "number" => {
            let initial = existing_value
                .as_f64()
                .map(|n| n.to_string())
                .unwrap_or_else(|| "0".to_string());
            loop {
                match inquire::Text::new("值").with_default(&initial).prompt() {
                    Ok(s) => match s.trim().parse::<f64>() {
                        Ok(n) => {
                            break serde_json::Value::Number(
                                serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()),
                            )
                        }
                        Err(_) => println!("请输入有效数字"),
                    },
                    Err(_) => return None,
                }
            }
        }
        "multiline" => match inquire::Editor::new("值")
            .with_predefined_text(&format_value(&existing_value))
            .prompt()
        {
            Ok(s) => serde_json::Value::String(s),
            Err(_) => return None,
        },
        _ => match inquire::Text::new("值")
            .with_default(&format_value(&existing_value))
            .prompt()
        {
            Ok(s) => serde_json::Value::String(s),
            Err(_) => return None,
        },
    };

    Some(serde_json::json!({
        "id": existing_obj
            .and_then(|o| o.get("id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        "name": name,
        "type": typ,
        "value": value,
    }))
}

fn default_value_for_type(typ: &str) -> serde_json::Value {
    match typ {
        "boolean" => serde_json::Value::Bool(false),
        "number" => serde_json::Value::Number(0.into()),
        "multiselect" => serde_json::Value::Array(vec![]),
        _ => serde_json::Value::String(String::new()),
    }
}

fn format_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => (if *b { "是" } else { "否" }).to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(arr) => {
            arr.iter().map(format_value).collect::<Vec<_>>().join(", ")
        }
        serde_json::Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// 安全退出 ratatui 备用屏幕与 raw 模式，回到普通终端。
fn leave_ratatui() -> Result<()> {
    disable_raw_mode()?;
    let _ = stdout().execute(DisableMouseCapture);
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(Show)?;
    stdout().flush()?;
    Ok(())
}

/// 重新进入 ratatui 备用屏幕与 raw 模式。
fn enter_ratatui() -> Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    stdout().execute(EnableMouseCapture)?;
    enable_raw_mode()?;
    Ok(())
}
