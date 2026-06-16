//! TUI 终端初始化与运行循环。

use std::io::{self, stdout};
use std::sync::Arc;
use std::time::Duration;

use color_eyre::Result;
use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use solosoul_core::VaultService;

use crate::app::{App, AppPhase};
use crate::events::poll_event;

pub struct Tui {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: App,
}

impl Tui {
    pub fn new(vault_service: VaultService) -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let app = App::new(Arc::new(vault_service))?;
        Ok(Self { terminal, app })
    }

    pub fn run(&mut self) -> Result<()> {
        // 进入备用屏幕、raw 模式并启用鼠标捕获
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(EnableMouseCapture)?;
        enable_raw_mode()?;
        self.terminal.clear()?;

        let result = self.run_loop();

        // 恢复终端
        disable_raw_mode()?;
        stdout().execute(DisableMouseCapture)?;
        stdout().execute(LeaveAlternateScreen)?;
        stdout().execute(Show)?;

        result
    }

    fn run_loop(&mut self) -> Result<()> {
        let tick_rate = Duration::from_millis(250);

        while !matches!(self.app.phase, AppPhase::Quit) {
            // 绘制一帧
            self.terminal.draw(|frame| self.app.render(frame))?;

            // 若存在需要 inquire 外部编辑的字段，优先处理
            if self.app.external_edit.is_some() {
                let request = self.app.external_edit.take().unwrap();
                match crate::widgets::external_editor::run(&request) {
                    Ok(value) => self.app.apply_external_edit(value),
                    Err(e) => {
                        self.app.error_message = Some(format!("外部编辑失败: {}", e));
                    }
                }
                // 恢复全屏后清屏并立即重绘，避免残影
                self.terminal.clear()?;
                continue;
            }

            // 轮询事件
            match poll_event(tick_rate)? {
                Some(event) => {
                    if self.app.handle_event(event)? {
                        break;
                    }
                }
                None => continue,
            }
        }

        Ok(())
    }
}

/// 恢复终端（用于 panic hook）。
pub fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    let _ = stdout().execute(DisableMouseCapture);
    stdout().execute(LeaveAlternateScreen)?;
    stdout().execute(Show)?;
    Ok(())
}
