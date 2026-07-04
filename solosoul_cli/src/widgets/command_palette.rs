//! 斜杠命令面板 —— 输入 `/` 后弹出的命令提示列表。
//!
//! 参考 Kimi Code CLI：输入 `/` 触发命令菜单，支持实时过滤、方向键选择、Enter 执行/填入。

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::commands::system::command_usage;
use crate::theme::Theme;
use crate::widgets::command_input::CommandInput;

/// 命令候选。
#[derive(Debug, Clone)]
pub struct CommandCandidate {
    pub command: &'static str,
    pub description: &'static str,
}

/// 面板处理结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    /// 未消费事件。
    None,
    /// 将命令填入输入框并关闭面板。
    Fill(&'static str),
    /// 唯一匹配时直接执行。
    Execute(&'static str),
    /// 关闭面板。
    Close,
}

/// 斜杠命令面板状态。
#[derive(Debug, Default, Clone)]
pub struct CommandPalette {
    /// 当前选中索引。
    selected: usize,
    /// 用户按 Esc 关闭后临时隐藏，直到输入再次变化。
    suppressed: bool,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self::default()
    }

    /// 重置选中位置与隐藏状态。
    pub fn reset(&mut self) {
        self.selected = 0;
        self.suppressed = false;
    }

    /// 临时隐藏面板（按 Esc 后）。
    pub fn suppress(&mut self) {
        self.suppressed = true;
    }

    /// 清除隐藏状态（输入变化后重新显示）。
    pub fn clear_suppress(&mut self) {
        self.suppressed = false;
    }

    /// 当前选中索引。
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// 是否应该渲染面板（未被临时隐藏）。
    pub fn should_render(&self, input: &CommandInput) -> bool {
        input.value.starts_with('/') && !self.suppressed
    }

    /// 根据当前阶段可用命令和输入值生成候选列表。
    pub fn build_candidates(commands: &[&'static str], input: &str) -> Vec<CommandCandidate> {
        let filter = input.trim().to_lowercase();
        commands
            .iter()
            .filter(|cmd| cmd.to_lowercase().starts_with(&filter))
            .map(|cmd| CommandCandidate {
                command: cmd,
                description: description_for(cmd),
            })
            .collect()
    }

    /// 处理键盘事件，返回动作与是否消费事件。
    pub fn handle_key(&mut self, key: &KeyEvent, candidates: &[CommandCandidate]) -> PaletteAction {
        if candidates.is_empty() {
            if key.code == KeyCode::Esc {
                return PaletteAction::Close;
            }
            return PaletteAction::None;
        }

        self.selected = self.selected.min(candidates.len() - 1);

        match key.code {
            KeyCode::Esc => {
                self.suppressed = true;
                PaletteAction::Close
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                PaletteAction::None
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(candidates.len() - 1);
                PaletteAction::None
            }
            KeyCode::Enter => {
                let selected_cmd = candidates[self.selected].command;
                if candidates.len() == 1 {
                    PaletteAction::Execute(selected_cmd)
                } else {
                    PaletteAction::Fill(selected_cmd)
                }
            }
            _ => PaletteAction::None,
        }
    }

    /// 渲染命令面板。
    ///
    /// `input_area` 为底部命令输入框区域，面板将贴在其上方。
    pub fn render(
        &self,
        frame: &mut ratatui::Frame,
        input_area: Rect,
        candidates: &[CommandCandidate],
    ) {
        let theme = Theme::load();
        let area = frame.area();

        let max_height =
            ((area.height.saturating_sub(1 + input_area.height)) as usize).clamp(3, 8) as u16;
        let width = input_area.width;
        let height = (candidates.len().max(1) as u16 + 2).min(max_height);
        let y = area
            .height
            .saturating_sub(input_area.height + height)
            .max(1);
        let palette_area = Rect::new(input_area.x, y, width, height);

        frame.render_widget(Clear, palette_area);

        if candidates.is_empty() {
            let paragraph = ratatui::widgets::Paragraph::new(
                Line::from("无匹配命令").style(theme.style_muted()),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.style_border(true))
                    .title(" 命令 ")
                    .title_style(theme.style_brand_dim()),
            );
            frame.render_widget(paragraph, palette_area);
            return;
        }

        let selected = self.selected.min(candidates.len() - 1);
        let visible_count = (height as usize).saturating_sub(2);
        let mut start = 0usize;
        if selected >= visible_count {
            start = selected - visible_count + 1;
        }
        let end = (start + visible_count).min(candidates.len());
        let visible = &candidates[start..end];

        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(idx, candidate)| {
                let global_idx = start + idx;
                let marker = if global_idx == selected { "> " } else { "  " };
                let line = Line::from(vec![
                    Span::styled(marker, theme.style_brand()),
                    Span::styled(candidate.command, theme.style_text()),
                    Span::styled("  ", theme.style_text()),
                    Span::styled(candidate.description, theme.style_hint()),
                ]);
                let mut item = ListItem::new(line);
                if global_idx == selected {
                    item = item.style(theme.style_card_focused());
                }
                item
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(selected));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.style_border(true))
                    .title(" 命令 ")
                    .title_style(theme.style_brand_dim()),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        frame.render_stateful_widget(list, palette_area, &mut state);
    }
}

/// 从 `command_usage` 中提取描述文本。
fn description_for(command: &str) -> &'static str {
    let usage = command_usage(command).unwrap_or("");
    let mut lines = usage.lines();
    let _ = lines.next(); // 第一行为用法
    for line in lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            // 去掉常见的前导 "  "
            return trimmed.trim_start_matches("  ");
        }
    }
    usage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent};

    fn candidates() -> Vec<CommandCandidate> {
        CommandPalette::build_candidates(&["/list", "/lock", "/logout"], "/lo")
    }

    #[test]
    fn test_should_render() {
        let palette = CommandPalette::new();
        let mut input = CommandInput::new();
        assert!(!palette.should_render(&input));
        input.set_value("/li".to_string());
        assert!(palette.should_render(&input));
    }

    #[test]
    fn test_filter_prefix() {
        let candidates = CommandPalette::build_candidates(&["/list", "/lock", "/size"], "/li");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].command, "/list");
    }

    #[test]
    fn test_filter_empty_matches_all() {
        let commands = &["/a", "/b"];
        let candidates = CommandPalette::build_candidates(commands, "/");
        assert_eq!(candidates.len(), 2);
    }

    #[test]
    fn test_handle_navigation() {
        let mut palette = CommandPalette::new();
        let candidates = candidates();
        palette.handle_key(&KeyEvent::from(KeyCode::Down), &candidates);
        assert_eq!(palette.selected, 1);
        palette.handle_key(&KeyEvent::from(KeyCode::Up), &candidates);
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn test_handle_fill_multiple_candidates() {
        let mut palette = CommandPalette::new();
        let candidates = candidates();
        let action = palette.handle_key(&KeyEvent::from(KeyCode::Enter), &candidates);
        assert_eq!(action, PaletteAction::Fill("/lock"));
    }

    #[test]
    fn test_handle_execute_unique_candidate() {
        let mut palette = CommandPalette::new();
        let candidates = CommandPalette::build_candidates(&["/list"], "/list");
        let action = palette.handle_key(&KeyEvent::from(KeyCode::Enter), &candidates);
        assert_eq!(action, PaletteAction::Execute("/list"));
    }

    #[test]
    fn test_handle_esc() {
        let mut palette = CommandPalette::new();
        let candidates = candidates();
        let action = palette.handle_key(&KeyEvent::from(KeyCode::Esc), &candidates);
        assert_eq!(action, PaletteAction::Close);
    }

    #[test]
    fn test_navigation_does_not_overflow() {
        let mut palette = CommandPalette::new();
        let candidates = candidates();
        palette.handle_key(&KeyEvent::from(KeyCode::Down), &candidates);
        palette.handle_key(&KeyEvent::from(KeyCode::Down), &candidates);
        palette.handle_key(&KeyEvent::from(KeyCode::Down), &candidates);
        assert_eq!(palette.selected, candidates.len() - 1);
    }

    #[test]
    fn test_render_empty_candidates_no_panic() {
        use ratatui::backend::TestBackend;

        let palette = CommandPalette::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                palette.render(frame, frame.area(), &[]);
            })
            .unwrap();
    }
}
