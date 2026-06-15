//! 底部命令输入框。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// 底部命令输入框状态。
#[derive(Debug, Default)]
pub struct CommandInput {
    /// 当前输入内容
    pub value: String,
    /// 光标在 value 中的字符位置（按 Unicode 标量值计数）
    pub cursor: usize,
}

impl CommandInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// 处理单个键盘事件，返回 true 表示该事件已被消费。
    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    match c {
                        'a' | 'A' => self.move_cursor_home(),
                        'e' | 'E' => self.move_cursor_end(),
                        'u' | 'U' => {
                            let _ = self.clear();
                        }
                        _ => return false,
                    }
                    true
                } else {
                    self.insert_char(c);
                    true
                }
            }
            KeyCode::Backspace => {
                self.backspace();
                true
            }
            KeyCode::Delete => {
                self.delete();
                true
            }
            KeyCode::Left => {
                self.move_cursor_left();
                true
            }
            KeyCode::Right => {
                self.move_cursor_right();
                true
            }
            KeyCode::Home => {
                self.move_cursor_home();
                true
            }
            KeyCode::End => {
                self.move_cursor_end();
                true
            }
            _ => false,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_pos = self.byte_position(self.cursor);
        self.value.insert(byte_pos, c);
        self.cursor += 1;
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let remove_at = self.cursor - 1;
        let byte_pos = self.byte_position(remove_at);
        let char_len = self.value[byte_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.value.drain(byte_pos..byte_pos + char_len);
        self.cursor -= 1;
    }

    pub fn delete(&mut self) {
        if self.cursor >= self.value.chars().count() {
            return;
        }
        let byte_pos = self.byte_position(self.cursor);
        let char_len = self.value[byte_pos..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
        self.value.drain(byte_pos..byte_pos + char_len);
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        let len = self.value.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    /// 设置输入内容并将光标移到末尾。
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor = self.value.chars().count();
    }

    /// 清空输入并返回清空前的内容。
    pub fn clear(&mut self) -> String {
        let value = std::mem::take(&mut self.value);
        self.cursor = 0;
        value
    }

    /// 尝试用候选命令补全当前输入。
    pub fn autocomplete(&mut self, candidates: &[&str]) {
        let current = self.value.trim();
        if current.is_empty() {
            return;
        }
        if let Some(candidate) = candidates
            .iter()
            .find(|c| c.starts_with(current) && **c != current)
        {
            self.value = candidate.to_string();
            self.cursor = self.value.chars().count();
        }
    }

    /// 渲染输入框为 ratatui Paragraph。
    pub fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::style::Style;
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let prefix = "> ";
        let line = Line::from(vec![
            Span::styled(prefix, Style::default().green().bold()),
            Span::raw(&self.value),
        ]);
        let paragraph =
            Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" 命令 "));
        frame.render_widget(paragraph, area);

        // 设置光标位置
        let cursor_x = area.x + prefix.len() as u16 + self.cursor as u16 + 1;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    /// 将字符索引转换为字节位置。
    fn byte_position(&self, char_index: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(idx, _)| idx)
            .unwrap_or(self.value.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_char(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    #[test]
    fn test_insert_and_cursor() {
        let mut input = CommandInput::new();
        input.handle_key(&key_char('h'));
        input.handle_key(&key_char('i'));
        assert_eq!(input.value, "hi");
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_backspace() {
        let mut input = CommandInput::new();
        input.handle_key(&key_char('a'));
        input.handle_key(&key_char('b'));
        input.handle_key(&KeyEvent::from(KeyCode::Backspace));
        assert_eq!(input.value, "a");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_move_cursor() {
        let mut input = CommandInput::new();
        input.handle_key(&key_char('a'));
        input.handle_key(&key_char('b'));
        input.handle_key(&KeyEvent::from(KeyCode::Left));
        assert_eq!(input.cursor, 1);
        input.handle_key(&KeyEvent::from(KeyCode::Home));
        assert_eq!(input.cursor, 0);
        input.handle_key(&KeyEvent::from(KeyCode::End));
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn test_autocomplete() {
        let mut input = CommandInput::new();
        input.handle_key(&key_char('/'));
        input.handle_key(&key_char('e'));
        input.autocomplete(&["/exit", "/edit"]);
        assert_eq!(input.value, "/exit");
    }
}
