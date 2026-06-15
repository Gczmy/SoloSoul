//! 全局状态机与 App 状态。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use solosoul_core::process_lock::ProcessLock;
use solosoul_core::{AccountSummary, VaultService};

use crate::commands;
use crate::widgets::command_input::CommandInput;

/// 当前所处界面/阶段。
#[derive(Debug, Clone)]
pub enum AppPhase {
    /// 无本地账户：提示用户先用 GUI 创建账户
    Welcome,
    /// 有账户但未登录：显示可用命令列表
    Locked,
    /// /account_list 结果页
    AccountList { accounts: Vec<AccountSummary> },
    /// /doctor 结果页
    Doctor {
        report: commands::doctor::DoctorReport,
    },
    /// 安全退出
    Quit,
}

pub struct App {
    pub phase: AppPhase,
    /// 返回 `Locked` 或 `Welcome` 时恢复此上一屏
    pub previous_phase: Option<AppPhase>,
    pub vault_service: Arc<VaultService>,
    pub process_lock: Option<ProcessLock>,
    pub command_input: CommandInput,
    pub command_history: Vec<String>,
    pub history_index: Option<usize>,
    pub last_activity: Instant,
    pub auto_lock_duration: Duration,
    /// 全局错误/消息 overlay，按任意键或 Esc 清除
    pub error_message: Option<String>,
    /// 日志文件路径，/doctor 中展示
    pub log_path: PathBuf,
}

impl App {
    pub fn new(vault_service: Arc<VaultService>) -> Result<Self> {
        let base_path = vault_service.base_path().to_path_buf();

        // 获取进程级排他锁
        let process_lock = match ProcessLock::acquire(&base_path) {
            Ok(lock) => Some(lock),
            Err(e) => {
                // Phase 1：仅记录警告，不阻塞启动（便于 doctor 展示状态）
                tracing::warn!("无法获取进程锁: {}", e);
                None
            }
        };

        let log_path = base_path.join("logs").join("cli.log");

        let phase = if vault_service.has_any_account() {
            AppPhase::Locked
        } else {
            AppPhase::Welcome
        };

        Ok(Self {
            phase,
            previous_phase: None,
            vault_service,
            process_lock,
            command_input: CommandInput::new(),
            command_history: Vec::new(),
            history_index: None,
            last_activity: Instant::now(),
            auto_lock_duration: Duration::from_secs(5 * 60),
            error_message: None,
            log_path,
        })
    }

    /// 处理事件，返回 true 表示应退出事件循环。
    pub fn handle_event(&mut self, event: crate::events::Event) -> Result<bool> {
        match event {
            crate::events::Event::Key(key) => self.handle_key(key),
            crate::events::Event::Tick => {
                self.last_activity = Instant::now();
                Ok(false)
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // 全局 Esc：先清 error overlay，再清命令框，再返回上一屏
        if key.code == KeyCode::Esc {
            if self.error_message.take().is_some() {
                return Ok(false);
            }
            if self.command_input.is_empty() {
                commands::core::back(self);
                return Ok(false);
            }
        }

        // 命令历史翻阅（仅当命令框激活且不在 autocomplete 模式时）
        if matches!(key.code, KeyCode::Up | KeyCode::Down) && self.command_input.is_empty() {
            self.handle_history(key.code);
            return Ok(false);
        }

        // 命令输入框优先消费事件
        if self.command_input.handle_key(&key) {
            return Ok(false);
        }

        // 全局快捷键
        if key.code == KeyCode::Enter {
            return self.execute_command();
        }

        if key.code == KeyCode::Tab {
            self.command_input
                .autocomplete(&["/account_list", "/back", "/doctor", "/exit"]);
            return Ok(false);
        }

        Ok(false)
    }

    fn handle_history(&mut self, code: KeyCode) {
        if self.command_history.is_empty() {
            return;
        }
        match code {
            KeyCode::Up => {
                let idx = self
                    .history_index
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(self.command_history.len() - 1);
                self.history_index = Some(idx);
                self.command_input.value = self.command_history[idx].clone();
                self.command_input.move_cursor_end();
            }
            KeyCode::Down => {
                if let Some(idx) = self.history_index {
                    if idx + 1 < self.command_history.len() {
                        self.history_index = Some(idx + 1);
                        self.command_input.value = self.command_history[idx + 1].clone();
                    } else {
                        self.history_index = None;
                        self.command_input.clear();
                    }
                    self.command_input.move_cursor_end();
                }
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) -> Result<bool> {
        let cmd = self.command_input.clear().trim().to_string();
        if cmd.is_empty() {
            return Ok(false);
        }
        self.command_history.push(cmd.clone());
        self.history_index = None;
        self.error_message = None;

        match cmd.as_str() {
            "/exit" => {
                commands::core::exit(self);
                return Ok(true);
            }
            "/back" => commands::core::back(self),
            "/account_list" => commands::auth::account_list(self)?,
            "/doctor" => commands::doctor::run(self)?,
            _ => {
                self.error_message = Some(format!("未知命令: {}", cmd));
            }
        }
        Ok(false)
    }

    /// 渲染一帧。
    pub fn render(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // 顶部状态栏
        let status_bar = crate::widgets::status_bar::render(self);
        frame.render_widget(status_bar, layout[0]);

        // 中间内容区
        match &self.phase {
            AppPhase::Welcome => crate::screens::welcome::render(frame, layout[1]),
            AppPhase::Locked => crate::screens::locked::render(frame, layout[1]),
            AppPhase::AccountList { accounts } => {
                crate::screens::account_list::render(frame, layout[1], accounts)
            }
            AppPhase::Doctor { report } => crate::screens::doctor::render(frame, layout[1], report),
            AppPhase::Quit => {}
        }

        // 底部命令输入框
        self.command_input.render(frame, layout[2]);

        // 全局错误 overlay
        if let Some(err) = &self.error_message {
            render_error_overlay(frame, err);
        }
    }
}

fn render_error_overlay(frame: &mut Frame, message: &str) {
    use ratatui::style::{Style, Stylize};
    use ratatui::text::{Line, Text};
    use ratatui::widgets::{Clear, Paragraph, Wrap};

    let area = frame.area();
    let width = (area.width as f32 * 0.6).min(60.0) as u16;
    let height = 3u16;
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = ratatui::layout::Rect::new(x, y, width, height);

    let text = Text::from(vec![
        Line::from("⚠ 错误").bold(),
        Line::from(message),
        Line::from("按 Esc 关闭").dark_gray(),
    ]);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: true })
        .style(Style::default().white().on_red())
        .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL));

    frame.render_widget(Clear, popup);
    frame.render_widget(paragraph, popup);
}
