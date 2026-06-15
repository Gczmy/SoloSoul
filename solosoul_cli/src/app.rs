//! 全局状态机与 App 状态。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use solosoul_core::process_lock::ProcessLock;
use solosoul_core::{AccountSummary, ObjectRecord, ObjectSummary, VaultService};

use crate::commands;
use crate::widgets::command_input::CommandInput;
use crate::widgets::password_input::PasswordInput;

/// 账户统计报告。
#[derive(Debug, Clone, Default)]
pub struct SizeReport {
    pub page_count: usize,
    pub object_count: usize,
    pub trash_count: usize,
    pub profile_count: usize,
    pub total_size_bytes: u64,
}

/// 当前所处界面/阶段。
// 状态机变体大小差异大是预期内（ObjectRecord 较大），可读性优先。
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum AppPhase {
    /// 无本地账户：提示用户先用 GUI 创建账户
    Welcome,
    /// 有账户但未登录：显示可用命令列表
    Locked,
    /// /account_list 结果页
    AccountList { accounts: Vec<AccountSummary> },
    /// 登录向导
    UnlockWizard { step: UnlockStep },
    /// 已登录：主界面
    Home { account_id: String },
    /// 页面/对象列表结果页
    ObjectList {
        items: Vec<ObjectSummary>,
        title: String,
    },
    /// 对象详情页
    ObjectDetail { object: ObjectRecord },
    /// 账户统计页
    Size { report: SizeReport },
    /// /doctor 结果页
    Doctor {
        report: commands::doctor::DoctorReport,
    },
    /// 安全退出
    Quit,
}

/// 登录向导步骤。
#[derive(Debug, Clone)]
pub enum UnlockStep {
    /// 选择账户
    SelectAccount {
        accounts: Vec<AccountSummary>,
        selected: usize,
    },
    /// 输入密码
    EnterPassword { account_id: String },
}

pub struct App {
    pub phase: AppPhase,
    /// 返回 `Locked` 或 `Welcome` 时恢复此上一屏
    pub previous_phase: Option<AppPhase>,
    pub vault_service: Arc<VaultService>,
    pub process_lock: Option<ProcessLock>,
    pub command_input: CommandInput,
    pub password_input: PasswordInput,
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
            password_input: PasswordInput::new(),
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
            crate::events::Event::Key(key) => {
                self.last_activity = Instant::now();
                self.handle_key(key)
            }
            crate::events::Event::Tick => self.handle_tick(),
        }
    }

    fn handle_tick(&mut self) -> Result<bool> {
        // 自动锁定检测
        if self.vault_service.is_unlocked() {
            let idle = Instant::now().duration_since(self.last_activity);
            if idle >= self.auto_lock_duration {
                self.vault_service.lock();
                self.password_input.clear();
                self.phase = AppPhase::Locked;
                self.error_message = Some("会话已超时锁定".to_string());
            }
        }
        Ok(false)
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // 全局 Esc：先清 error overlay
        if key.code == KeyCode::Esc && self.error_message.take().is_some() {
            return Ok(false);
        }

        // 根据当前阶段分发
        match &self.phase.clone() {
            AppPhase::UnlockWizard { step } => self.handle_unlock_key(key, step.clone()),
            _ => self.handle_command_key(key),
        }
    }

    /// 登录向导的键盘处理。
    fn handle_unlock_key(&mut self, key: KeyEvent, step: UnlockStep) -> Result<bool> {
        match step {
            UnlockStep::SelectAccount {
                accounts,
                mut selected,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        commands::core::back(self);
                    }
                    KeyCode::Up if selected > 0 => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down if selected + 1 < accounts.len() => {
                        selected += 1;
                    }
                    KeyCode::Enter => {
                        let account_id = accounts[selected].id.clone();
                        self.phase = AppPhase::UnlockWizard {
                            step: UnlockStep::EnterPassword { account_id },
                        };
                        self.password_input.clear();
                        return Ok(false);
                    }
                    _ => {}
                }
                self.phase = AppPhase::UnlockWizard {
                    step: UnlockStep::SelectAccount { accounts, selected },
                };
                Ok(false)
            }
            UnlockStep::EnterPassword { account_id } => {
                if key.code == KeyCode::Esc {
                    self.password_input.clear();
                    commands::core::back(self);
                    return Ok(false);
                }

                if key.code == KeyCode::Enter {
                    self.submit_password(&account_id)?;
                    return Ok(false);
                }

                self.password_input.handle_key(&key);
                Ok(false)
            }
        }
    }

    /// 提交密码进行解锁。
    fn submit_password(&mut self, account_id: &str) -> Result<()> {
        let password = self.password_input.value().clone();
        self.password_input.clear();

        match self.vault_service.unlock_secure(account_id, &password) {
            Ok(()) => {
                drop(password);
                self.phase = AppPhase::Home {
                    account_id: account_id.to_string(),
                };
            }
            Err(e) => {
                drop(password);
                self.error_message = Some(format!("登录失败: {}", e));
            }
        }
        Ok(())
    }

    /// 普通命令模式键盘处理。
    fn handle_command_key(&mut self, key: KeyEvent) -> Result<bool> {
        // 全局 Esc：先清 error overlay，再清命令框，再返回上一屏
        if key.code == KeyCode::Esc {
            if self.command_input.is_empty() {
                commands::core::back(self);
            }
            return Ok(false);
        }

        // 命令历史翻阅（仅当命令框为空时）
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
                .autocomplete(available_commands(&self.phase));
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

        let parts: Vec<&str> = cmd.split_whitespace().collect();
        let base = parts.first().copied().unwrap_or("");

        match base {
            "/exit" => {
                commands::core::exit(self);
                return Ok(true);
            }
            "/back" => commands::core::back(self),
            "/account_list" => commands::auth::account_list(self)?,
            "/doctor" => commands::doctor::run(self)?,
            "/unlock" => commands::auth::unlock(self)?,
            "/lock" | "/logout" => commands::auth::lock(self),
            "/list" => commands::vault_read::list(self, parts.get(1).copied())?,
            "/open" => commands::vault_read::open(self, parts.get(1).copied())?,
            "/size" => commands::vault_read::size(self)?,
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
            AppPhase::UnlockWizard { step } => {
                crate::screens::unlock::render(frame, layout[1], step, &self.password_input)
            }
            AppPhase::Home { account_id } => {
                crate::screens::home::render(frame, layout[1], account_id)
            }
            AppPhase::ObjectList { items, title } => {
                crate::screens::object_list::render(frame, layout[1], title, items)
            }
            AppPhase::ObjectDetail { object } => {
                crate::screens::object_detail::render(frame, layout[1], object)
            }
            AppPhase::Size { report } => crate::screens::size::render(frame, layout[1], report),
            AppPhase::Doctor { report } => crate::screens::doctor::render(frame, layout[1], report),
            AppPhase::Quit => {}
        }

        // 底部命令输入框（登录向导的密码页除外）
        if !matches!(
            self.phase,
            AppPhase::UnlockWizard {
                step: UnlockStep::EnterPassword { .. }
            }
        ) {
            self.command_input.render(frame, layout[2]);
        }

        // 全局错误 overlay
        if let Some(err) = &self.error_message {
            render_error_overlay(frame, err);
        }
    }
}

/// 根据当前阶段返回可用的命令补全候选。
fn available_commands(phase: &AppPhase) -> &'static [&'static str] {
    match phase {
        AppPhase::Welcome | AppPhase::Locked => {
            &["/account_list", "/back", "/doctor", "/exit", "/unlock"]
        }
        AppPhase::Home { .. } => &[
            "/account_list",
            "/back",
            "/doctor",
            "/exit",
            "/lock",
            "/logout",
            "/list",
            "/open",
            "/size",
        ],
        AppPhase::UnlockWizard { .. } => &["/back"],
        // 其他结果页至少支持 /back 和 /exit
        _ => &["/back", "/exit"],
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use crossterm::event::{KeyCode, KeyEvent};

    use super::*;

    fn locked_app() -> (App, String, tempfile::TempDir) {
        let _guard = crate::VAULT_TEST_LOCK.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("SOLOSOUL_DATA_DIR", dir.path());
        let vault = VaultService::new();
        let account = vault.create_account("Test", "password123", None).unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        vault.lock();
        let app = App::new(Arc::new(vault)).unwrap();
        (app, account_id, dir)
    }

    #[test]
    fn test_available_commands_locked() {
        assert_eq!(
            super::available_commands(&AppPhase::Locked),
            &["/account_list", "/back", "/doctor", "/exit", "/unlock"]
        );
    }

    #[test]
    fn test_available_commands_home() {
        assert!(super::available_commands(&AppPhase::Home {
            account_id: "acc".to_string()
        })
        .contains(&"/list"));
    }

    #[test]
    fn test_unlock_success_and_lock() {
        let (mut app, account_id, _dir) = locked_app();
        assert!(matches!(app.phase, AppPhase::Locked));

        // 进入登录向导（单账户直接到密码输入）
        commands::auth::unlock(&mut app).unwrap();
        assert!(matches!(
            app.phase,
            AppPhase::UnlockWizard {
                step: UnlockStep::EnterPassword { .. }
            }
        ));

        // 输入密码
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        assert!(matches!(app.phase, AppPhase::Home { .. }));
        assert_eq!(app.vault_service.get_current_account(), Some(account_id));

        // 锁定
        commands::auth::lock(&mut app);
        assert!(matches!(app.phase, AppPhase::Locked));
        assert!(!app.vault_service.is_unlocked());
    }

    #[test]
    fn test_unlock_wrong_password() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();

        for c in "wrongpass".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        assert!(!app.vault_service.is_unlocked());
        assert!(app.error_message.is_some());
    }

    #[test]
    fn test_auto_lock() {
        let (mut app, _id, _dir) = locked_app();
        // 先手动解锁
        commands::auth::unlock(&mut app).unwrap();
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();
        assert!(app.vault_service.is_unlocked());

        // 模拟超时无操作
        app.last_activity = Instant::now() - Duration::from_secs(400);
        app.handle_event(crate::events::Event::Tick).unwrap();

        assert!(!app.vault_service.is_unlocked());
        assert!(matches!(app.phase, AppPhase::Locked));
    }

    #[test]
    fn test_render_home_does_not_panic() {
        let (mut app, _id, _dir) = locked_app();
        commands::auth::unlock(&mut app).unwrap();
        for c in "password123".chars() {
            app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Char(c))))
                .unwrap();
        }
        app.handle_event(crate::events::Event::Key(KeyEvent::from(KeyCode::Enter)))
            .unwrap();

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
    }
}
