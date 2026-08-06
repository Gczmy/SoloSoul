//! 模态提示组件（替代临时 `inquire` 弹窗）。
//!
//! 在 TUI 内以 overlay 形式提供文本输入、单选、确认三种提示。
//! 打开提示时会自动暂停自动锁定计时，关闭后恢复。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use zeroize::Zeroizing;

use crate::app::App;

/// 提示类型定义。
#[derive(Debug, Clone)]
pub enum PromptSpec {
    /// 单行文本输入。
    Text {
        label: String,
        initial: String,
        /// 是否默认隐藏输入（用于敏感字段）。
        mask: bool,
        /// 是否允许按 Tab 切换显示/隐藏。
        allow_toggle_mask: bool,
    },
    /// 单选列表。
    Select {
        label: String,
        options: Vec<String>,
        selected: usize,
    },
    /// 确认（Y/n）。
    Confirm {
        message: String,
        /// true = 默认 Y，false = 默认 n。
        default_yes: bool,
    },
}

/// 提示结果。
///
/// `Text` 用 `Zeroizing<String>` 承载：mask 提示（主密码/导出密码等）的输入
/// 在回调消费完成后随 drop 自动清零，与 `PasswordInput` 的零化约定一致。
#[derive(Debug, Clone)]
pub enum PromptResult {
    Text(Zeroizing<String>),
    Select(usize),
    Confirm(bool),
    Cancel,
}

/// 提示完成回调类型。
pub type PromptCallback = Box<dyn FnOnce(&mut App, PromptResult)>;

/// 当前提示状态。
pub struct PromptState {
    pub spec: PromptSpec,
    /// 文本输入内容（零化承载，drop 时自动清零）
    pub value: Zeroizing<String>,
    /// 文本光标位置（字符索引）
    pub cursor: usize,
    /// 列表当前选中
    pub selected: usize,
    /// 文本是否处于掩码状态
    pub mask: bool,
    /// 完成回调
    pub on_done: PromptCallback,
}

impl PromptState {
    fn new_text(
        label: String,
        initial: String,
        mask: bool,
        allow_toggle_mask: bool,
        on_done: PromptCallback,
    ) -> Self {
        let cursor = initial.chars().count();
        Self {
            spec: PromptSpec::Text {
                label,
                initial: initial.clone(),
                mask,
                allow_toggle_mask,
            },
            value: Zeroizing::new(initial),
            cursor,
            selected: 0,
            mask,
            on_done,
        }
    }

    fn new_select(
        label: String,
        options: Vec<String>,
        selected: usize,
        on_done: PromptCallback,
    ) -> Self {
        Self {
            spec: PromptSpec::Select {
                label,
                options,
                selected,
            },
            value: Zeroizing::new(String::new()),
            cursor: 0,
            selected,
            mask: false,
            on_done,
        }
    }

    fn new_confirm(message: String, default_yes: bool, on_done: PromptCallback) -> Self {
        Self {
            spec: PromptSpec::Confirm {
                message,
                default_yes,
            },
            value: Zeroizing::new(String::new()),
            cursor: 0,
            selected: if default_yes { 0 } else { 1 },
            mask: false,
            on_done,
        }
    }
}

/// 打开一个提示，并暂停自动锁定。
pub fn open(app: &mut App, spec: PromptSpec, on_done: PromptCallback) {
    app.auto_lock_paused = true;
    let state = match spec {
        PromptSpec::Text {
            label,
            initial,
            mask,
            allow_toggle_mask,
        } => PromptState::new_text(label, initial, mask, allow_toggle_mask, on_done),
        PromptSpec::Select {
            label,
            options,
            selected,
        } => PromptState::new_select(label, options, selected, on_done),
        PromptSpec::Confirm {
            message,
            default_yes,
        } => PromptState::new_confirm(message, default_yes, on_done),
    };
    app.prompt = Some(state);
}

/// 完成提示并调用回调。
fn finish(app: &mut App, result: PromptResult) {
    // 取出 on_done 后关闭提示，避免回调中再次打开提示时造成双重借用。
    if let Some(state) = app.prompt.take() {
        app.auto_lock_paused = false;
        app.last_activity = std::time::Instant::now();
        (state.on_done)(app, result);
    }
}

/// 处理提示相关的键盘事件；返回 true 表示事件已被消费。
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let Some(state) = app.prompt.as_mut() else {
        return false;
    };

    match &state.spec {
        PromptSpec::Text {
            allow_toggle_mask, ..
        } => match key.code {
            KeyCode::Esc => {
                finish(app, PromptResult::Cancel);
                true
            }
            KeyCode::Enter => {
                let value = state.value.clone();
                finish(app, PromptResult::Text(value));
                true
            }
            KeyCode::Tab if *allow_toggle_mask => {
                state.mask = !state.mask;
                true
            }
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'a' | 'A' => state.cursor = 0,
                        'e' | 'E' => state.cursor = state.value.chars().count(),
                        'u' | 'U' => {
                            state.value.clear();
                            state.cursor = 0;
                        }
                        _ => return false,
                    }
                } else {
                    let pos = byte_position(&state.value, state.cursor);
                    state.value.insert(pos, c);
                    state.cursor += 1;
                }
                true
            }
            KeyCode::Backspace => {
                if state.cursor > 0 {
                    let remove_at = state.cursor - 1;
                    let byte_pos = byte_position(&state.value, remove_at);
                    let char_len = state.value[byte_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    state.value.drain(byte_pos..byte_pos + char_len);
                    state.cursor -= 1;
                }
                true
            }
            KeyCode::Delete => {
                let len = state.value.chars().count();
                if state.cursor < len {
                    let byte_pos = byte_position(&state.value, state.cursor);
                    let char_len = state.value[byte_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(1);
                    state.value.drain(byte_pos..byte_pos + char_len);
                }
                true
            }
            KeyCode::Left => {
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
                true
            }
            KeyCode::Right => {
                let len = state.value.chars().count();
                if state.cursor < len {
                    state.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                state.cursor = 0;
                true
            }
            KeyCode::End => {
                state.cursor = state.value.chars().count();
                true
            }
            _ => false,
        },
        PromptSpec::Select { options, .. } => match key.code {
            KeyCode::Esc => {
                finish(app, PromptResult::Cancel);
                true
            }
            KeyCode::Enter => {
                let selected = state.selected;
                finish(app, PromptResult::Select(selected));
                true
            }
            KeyCode::Up => {
                if state.selected > 0 {
                    state.selected -= 1;
                }
                true
            }
            KeyCode::Down => {
                if state.selected + 1 < options.len() {
                    state.selected += 1;
                }
                true
            }
            _ => false,
        },
        PromptSpec::Confirm {
            default_yes: _default_yes,
            ..
        } => {
            match key.code {
                KeyCode::Esc => {
                    finish(app, PromptResult::Confirm(false));
                    true
                }
                KeyCode::Enter => {
                    let yes = state.selected == 0;
                    finish(app, PromptResult::Confirm(yes));
                    true
                }
                KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                    state.selected = 1 - state.selected;
                    true
                }
                KeyCode::Char(c) => {
                    match c.to_ascii_lowercase() {
                        'y' => {
                            finish(app, PromptResult::Confirm(true));
                            return true;
                        }
                        'n' => {
                            finish(app, PromptResult::Confirm(false));
                            return true;
                        }
                        _ => {}
                    }
                    // 方向键已处理，其他字符忽略。
                    false
                }
                KeyCode::Up | KeyCode::Down => {
                    state.selected = 1 - state.selected;
                    true
                }
                _ => false,
            }
        }
    }
}

use crate::util::byte_position;

/// 渲染提示 overlay。
pub fn render(app: &App, frame: &mut ratatui::Frame) {
    let Some(state) = app.prompt.as_ref() else {
        return;
    };

    let area = frame.area();
    let popup = centered_rect(70, 40, area);
    frame.render_widget(Clear, popup);

    let inner = popup.inner(Margin::new(2, 1));
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

    match &state.spec {
        PromptSpec::Text {
            label,
            allow_toggle_mask,
            ..
        } => {
            let title = format!(" {} ", label);
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().cyan());
            frame.render_widget(block, popup);

            let display = if state.mask {
                "•".repeat(state.value.chars().count())
            } else {
                state.value.to_string()
            };
            let line = Line::from(vec![Span::raw(display)]);
            let input = Paragraph::new(line)
                .block(Block::default().borders(Borders::BOTTOM))
                .alignment(Alignment::Left);
            frame.render_widget(input, layout[1]);

            let mut hint = String::from("Enter 确认 · Esc 取消");
            if *allow_toggle_mask {
                hint.push_str(" · Tab 切换掩码");
            }
            frame.render_widget(
                Paragraph::new(Line::from(hint).dark_gray()).alignment(Alignment::Center),
                layout[2],
            );

            // 光标
            let cursor_x = layout[1].x + 1 + state.cursor as u16;
            let cursor_y = layout[1].y + 1;
            frame.set_cursor_position((cursor_x, cursor_y));
        }
        PromptSpec::Select { label, options, .. } => {
            let title = format!(" {} ", label);
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().cyan());
            frame.render_widget(block, popup);

            let lines: Vec<Line> = options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let marker = if i == state.selected { "▸ " } else { "  " };
                    if i == state.selected {
                        Line::from(vec![
                            Span::raw(marker),
                            Span::styled(opt, Style::default().bold().reversed()),
                        ])
                    } else {
                        Line::from(vec![Span::raw(marker), Span::raw(opt)])
                    }
                })
                .collect();
            let text = Text::from(lines);
            frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), layout[1]);

            frame.render_widget(
                Paragraph::new(Line::from("↑/↓ 选择 · Enter 确认 · Esc 取消").dark_gray())
                    .alignment(Alignment::Center),
                layout[2],
            );
        }
        PromptSpec::Confirm {
            message,
            default_yes: _,
        } => {
            let block = Block::default()
                .title(" 确认 ")
                .borders(Borders::ALL)
                .border_style(Style::default().yellow());
            frame.render_widget(block, popup);

            let yes_style = if state.selected == 0 {
                Style::default().bold()
            } else {
                Style::default()
            };
            let no_style = if state.selected == 1 {
                Style::default().bold()
            } else {
                Style::default()
            };
            let yes_marker = if state.selected == 0 { "▸ " } else { "  " };
            let no_marker = if state.selected == 1 { "▸ " } else { "  " };
            let line = Line::from(vec![
                Span::raw(message.clone()),
                Span::raw("  "),
                Span::styled(format!("{}[Y]", yes_marker), yes_style),
                Span::raw(" / "),
                Span::styled(format!("{}[n]", no_marker), no_style),
            ]);
            let para = Paragraph::new(line)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            frame.render_widget(para, layout[1]);

            frame.render_widget(
                Paragraph::new(Line::from("Enter 确认 · Esc/n 取消 · Tab 切换").dark_gray())
                    .alignment(Alignment::Center),
                layout[2],
            );
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let width = (r.width * percent_x / 100).max(20).min(r.width);
    let height = (r.height * percent_y / 100).max(7).min(r.height);
    let x = (r.width.saturating_sub(width)) / 2 + r.x;
    let y = (r.height.saturating_sub(height)) / 2 + r.y;
    Rect::new(x, y, width, height)
}
