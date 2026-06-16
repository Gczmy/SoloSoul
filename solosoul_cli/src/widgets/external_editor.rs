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

/// 在终端普通模式下运行 inquire，返回新的字段值；用户取消返回 `Ok(None)`。
pub fn run(request: &ExternalEditRequest) -> Result<Option<serde_json::Value>> {
    leave_ratatui()?;
    let result = match request {
        ExternalEditRequest::Date(field) => edit_date(field),
        ExternalEditRequest::DateTime(field) => edit_datetime(field),
        ExternalEditRequest::Select(field) => edit_select(field),
        ExternalEditRequest::MultiSelect(field) => edit_multi_select(field),
        ExternalEditRequest::Textarea(field) => edit_textarea(field),
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
