//! 密码输入框（掩码显示）。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use zeroize::Zeroizing;

/// 密码输入框状态。
///
/// 内部使用 `Zeroizing<String>`，离开作用域后自动清零。
#[derive(Debug, Default)]
pub struct PasswordInput {
    value: Zeroizing<String>,
    /// 光标在 value 中的字符位置
    cursor: usize,
}

impl PasswordInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// 返回当前密码值的引用。
    pub fn value(&self) -> &Zeroizing<String> {
        &self.value
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
                            self.clear();
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

    fn insert_char(&mut self, c: char) {
        let byte_pos = self.byte_position(self.cursor);
        self.value.insert(byte_pos, c);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
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

    fn delete(&mut self) {
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

    fn move_cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn move_cursor_right(&mut self) {
        let len = self.value.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    fn move_cursor_home(&mut self) {
        self.cursor = 0;
    }

    fn move_cursor_end(&mut self) {
        self.cursor = self.value.chars().count();
    }

    /// 清空输入并 zeroize 旧内容。
    pub fn clear(&mut self) {
        self.value = Zeroizing::new(String::new());
        self.cursor = 0;
    }

    /// 渲染输入框为 ratatui Paragraph，所有字符显示为 `•`。
    pub fn render(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph};

        let theme = crate::theme::Theme::load();
        let prefix = "密码: ";
        let masked: String = self.value.chars().map(|_| '•').collect();
        let line = Line::from(vec![
            Span::styled(prefix, theme.style_brand()),
            Span::styled(masked, theme.style_text()),
        ]);
        let paragraph = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 登录 ")
                .title_style(theme.style_brand_dim())
                .border_style(theme.style_border(false)),
        );
        frame.render_widget(paragraph, area);

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
    fn test_insert_and_mask() {
        let mut input = PasswordInput::new();
        input.handle_key(&key_char('s'));
        input.handle_key(&key_char('e'));
        input.handle_key(&key_char('c'));
        assert_eq!(input.value().as_str(), "sec");
        assert_eq!(input.cursor, 3);
    }

    #[test]
    fn test_backspace_and_cursor() {
        let mut input = PasswordInput::new();
        input.handle_key(&key_char('a'));
        input.handle_key(&key_char('b'));
        input.handle_key(&KeyEvent::from(KeyCode::Backspace));
        assert_eq!(input.value().as_str(), "a");
        assert_eq!(input.cursor, 1);
    }

    #[test]
    fn test_move_cursor() {
        let mut input = PasswordInput::new();
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
    fn test_clear_zeroizes() {
        let mut input = PasswordInput::new();
        input.handle_key(&key_char('x'));
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor, 0);
    }
}
