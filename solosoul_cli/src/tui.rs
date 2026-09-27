//! TUI 终端初始化与运行循环。

use std::io::{self, stdout};
use std::sync::Arc;
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::cursor::Show;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use solosoul_core::VaultService;

use crate::app::{App, AppPhase};
use crate::events::{poll_event, Event};

const TICK_RATE: Duration = Duration::from_millis(250);
const TASK_EVENTS_PER_TURN: usize = 32;

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
        // 在第一次终端操作前建立 guard，初始化失败和 unwind 也能恢复已改变的状态。
        let mut guard = TerminalRestoreGuard::new(restore_terminal);
        let result = match self.initialize_terminal() {
            Ok(()) => self.run_loop(),
            Err(error) => finish_tasks(&mut self.app, Err(error)),
        };
        preserve_primary(result, guard.restore(), "terminal restoration")
    }

    fn initialize_terminal(&mut self) -> Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(EnableMouseCapture)?;
        enable_raw_mode()?;
        self.terminal.clear()?;
        Ok(())
    }

    fn run_loop(&mut self) -> Result<()> {
        run_loop_with(&mut self.terminal, &mut self.app, poll_event, Instant::now)
    }
}

/// 生产和 TestBackend 共用同一驱动；poll/时钟仅隔离终端输入与真实等待。
/// 每轮独立检查 Tick 截止时间，任务进度和连续 Resize 都不能饿死自动锁定。
fn run_loop_with<B, P, N>(terminal: &mut Terminal<B>, app: &mut App, poll: P, now: N) -> Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    P: FnMut(Duration) -> Result<Option<Event>>,
    N: FnMut() -> Instant,
{
    let result = drive_loop(terminal, app, poll, now);
    // 正常退出、poll/draw/事件错误均取消并等待实际 Future 回收。
    finish_tasks(app, result)
}

fn drive_loop<B, P, N>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mut poll: P,
    mut now: N,
) -> Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
    P: FnMut(Duration) -> Result<Option<Event>>,
    N: FnMut() -> Instant,
{
    let mut next_tick = now() + TICK_RATE;
    while !matches!(app.phase, AppPhase::Quit) {
        let current = now();
        if current >= next_tick {
            if app.handle_event_at(Event::Tick, current)? {
                break;
            }
            // 慢的既有同步命令返回后只补一个 Tick，避免追赶历史 Tick 阻塞输入。
            next_tick = current + TICK_RATE;
        }
        // 到期锁定先于结果应用；只消费有界批次，终端输入和绘制每轮都有机会。
        app.drain_task_events(TASK_EVENTS_PER_TURN)?;
        if matches!(app.phase, AppPhase::Quit) {
            break;
        }
        terminal.draw(|frame| app.render(frame))?;

        // 保留 inquire 外部编辑的既有同步语义；RF211 不迁移其他业务命令。
        if let Some(request) = app.external_edit.take() {
            match crate::widgets::external_editor::run(&request) {
                Ok(value) => app.apply_external_edit(value),
                Err(error) => app.error_message = Some(format!("外部编辑失败: {error}")),
            }
            terminal.clear()?;
            continue;
        }

        let timeout = next_tick.saturating_duration_since(now());
        if let Some(event) = poll(timeout)? {
            // poll 的超时 Tick 仅用于唤醒；下一轮按同一 deadline 派发一次。
            if !matches!(event, Event::Tick) && app.handle_event_at(event, now())? {
                break;
            }
        }
    }
    Ok(())
}

fn finish_tasks(app: &mut App, result: Result<()>) -> Result<()> {
    preserve_primary(result, app.shutdown_tasks(), "task shutdown")
}

fn preserve_primary(result: Result<()>, cleanup: Result<()>, stage: &str) -> Result<()> {
    match (result, cleanup) {
        (Err(primary), Err(secondary)) => {
            tracing::warn!("{stage} also failed: {secondary}");
            Err(primary)
        }
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), cleanup) => cleanup,
    }
}

struct TerminalRestoreGuard<F: FnMut() -> Result<()>> {
    restore: F,
    armed: bool,
}

impl<F: FnMut() -> Result<()>> TerminalRestoreGuard<F> {
    fn new(restore: F) -> Self {
        Self {
            restore,
            armed: true,
        }
    }

    fn restore(&mut self) -> Result<()> {
        if self.armed {
            self.armed = false;
            (self.restore)()
        } else {
            Ok(())
        }
    }
}

impl<F: FnMut() -> Result<()>> Drop for TerminalRestoreGuard<F> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

/// 在 TUI 所在线程安装：后台异常由任务所有者处理，不应改变仍运行的终端。
pub fn install_panic_hook() {
    install_panic_hook_with(restore_terminal);
}

fn install_panic_hook_with<F>(restore: F)
where
    F: Fn() -> Result<()> + Send + Sync + 'static,
{
    let tui_thread = std::thread::current().id();
    std::panic::set_hook(Box::new(move |_info| {
        // 不读取 payload/位置，也不调用旧 hook，避免正文经 stderr 或日志泄露。
        if std::thread::current().id() == tui_thread {
            let _ = restore();
            eprintln!("CLI terminated after an internal error");
        } else {
            tracing::error!("CLI background task panicked");
        }
    }));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RestoreStep {
    RawMode,
    MouseCapture,
    AlternateScreen,
    Cursor,
}

fn restore_terminal_with(mut restore: impl FnMut(RestoreStep) -> Result<()>) -> Result<()> {
    let mut result = Ok(());
    for step in [
        RestoreStep::RawMode,
        RestoreStep::MouseCapture,
        RestoreStep::AlternateScreen,
        RestoreStep::Cursor,
    ] {
        result = preserve_primary(result, restore(step), "terminal restoration");
    }
    result
}

/// 恢复终端（也供 panic hook 使用）；单项失败仍继续后续步骤。
pub fn restore_terminal() -> Result<()> {
    restore_terminal_with(|step| {
        match step {
            RestoreStep::RawMode => disable_raw_mode()?,
            RestoreStep::MouseCapture => {
                stdout().execute(DisableMouseCapture)?;
            }
            RestoreStep::AlternateScreen => {
                stdout().execute(LeaveAlternateScreen)?;
            }
            RestoreStep::Cursor => {
                stdout().execute(Show)?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
#[path = "tui/rf211_tests.rs"]
mod rf211_tests;

#[cfg(test)]
#[path = "tui/panic_tests.rs"]
mod panic_tests;
