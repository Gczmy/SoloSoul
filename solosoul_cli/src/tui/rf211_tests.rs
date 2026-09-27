//! RF211：通过生产循环、真实任务所有者与 TestBackend 验证公平调度和退出回收。

use std::cell::Cell as ClockCell;
use std::io;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::backend::{Backend, ClearType, TestBackend, WindowSize};
use ratatui::buffer::Cell;
use ratatui::layout::{Position, Size};
use ratatui::Terminal;
use solosoul_core::VaultService;
use tokio::sync::oneshot;

use super::{restore_terminal_with, run_loop_with, RestoreStep, TerminalRestoreGuard};
use crate::app::{App, AppPhase};
use crate::events::Event;
use crate::tasks::TaskOutput;

const WAIT: Duration = Duration::from_secs(15);

struct Fixture {
    app: App,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::TempDir::new().unwrap();
        let service = Arc::new(VaultService::with_base_path(dir.path().to_path_buf()));
        let account = service
            .create_account("RF211 TUI synthetic", crate::TEST_PASSWORD, None)
            .unwrap();
        let account_id = account["id"].as_str().unwrap().to_string();
        let mut app = App::new(service).unwrap();
        app.phase = AppPhase::Home { account_id };
        app.account_name = "RF211 TUI synthetic".to_string();
        app.i18n.set_locale("en-US");
        Self { app, _dir: dir }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.app.shutdown_tasks();
    }
}

/// 捕获在实际 Future 中的资源；只有 Future 已析构才确认回收。
struct Resource(mpsc::Sender<()>);

impl Drop for Resource {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

struct PendingTask {
    release: Option<oneshot::Sender<()>>,
    dropped: mpsc::Receiver<()>,
}

impl PendingTask {
    fn start(app: &mut App) -> Self {
        let account = app.vault_service.get_current_account().unwrap();
        let session = app.vault_service.capture_session(&account).unwrap();
        let (release, released) = oneshot::channel();
        let (started, ready) = mpsc::channel();
        let (dropped, observed) = mpsc::channel();
        let task = Self {
            release: Some(release),
            dropped: observed,
        };
        app.tasks
            .spawn(session, move |_context| async move {
                let _resource = Resource(dropped);
                let _ = started.send(());
                let _ = released.await;
                Ok(TaskOutput::Message("synthetic completed body".to_string()))
            })
            .unwrap();
        ready
            .recv_timeout(WAIT)
            .expect("managed task must reach its async barrier");
        task
    }

    fn assert_running(&self) {
        assert!(matches!(
            self.dropped.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    }

    fn assert_reclaimed(&self) {
        self.dropped
            .recv_timeout(WAIT)
            .expect("loop return must follow actual Future drop");
    }
}

impl Drop for PendingTask {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

/// 保留 TestBackend 的真实绘制，仅在指定 draw 调用注入 I/O 故障。
struct ObservedBackend {
    inner: TestBackend,
    frames: Arc<Mutex<Vec<String>>>,
    draw_count: usize,
    fail_draw: Option<usize>,
}

impl ObservedBackend {
    fn new(frames: Arc<Mutex<Vec<String>>>, fail_draw: Option<usize>) -> Self {
        Self {
            inner: TestBackend::new(110, 26),
            frames,
            draw_count: 0,
            fail_draw,
        }
    }
}

impl Backend for ObservedBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.draw_count += 1;
        if self.fail_draw == Some(self.draw_count) {
            return Err(io::Error::other("RF211 synthetic draw failure"));
        }
        self.inner.draw(content).unwrap();
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor().unwrap();
        Ok(())
    }
    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor().unwrap();
        Ok(())
    }
    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(self.inner.get_cursor_position().unwrap())
    }
    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position).unwrap();
        Ok(())
    }
    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear().unwrap();
        Ok(())
    }
    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.inner.clear_region(clear_type).unwrap();
        Ok(())
    }
    fn size(&self) -> io::Result<Size> {
        Ok(self.inner.size().unwrap())
    }
    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(self.inner.window_size().unwrap())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush().unwrap();
        self.frames.lock().unwrap().push(self.inner.to_string());
        Ok(())
    }
}

fn key(code: KeyCode) -> Option<Event> {
    Some(Event::Key(KeyEvent::from(code)))
}

#[test]
fn rf211_slow_managed_task_keeps_real_loop_keys_frames_and_ticks_moving() {
    let _lock = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new();
    let task = PendingTask::start(&mut fixture.app);
    let frames = Arc::new(Mutex::new(Vec::new()));
    let mut terminal = Terminal::new(ObservedBackend::new(Arc::clone(&frames), None)).unwrap();
    let base = Instant::now();
    fixture.app.last_activity = base;
    let clock = ClockCell::new(base);
    let original_sheen = fixture.app.sheen_offset;
    let mut events = [
        key(KeyCode::Char('x')),
        None,
        None,
        key(KeyCode::Backspace),
        key(KeyCode::Char('/')),
        key(KeyCode::Char('e')),
        key(KeyCode::Char('x')),
        key(KeyCode::Char('i')),
        key(KeyCode::Char('t')),
        key(KeyCode::Enter),
    ]
    .into_iter();
    let mut polls = 0;
    run_loop_with(
        &mut terminal,
        &mut fixture.app,
        |timeout| {
            assert!(timeout <= Duration::from_millis(250));
            task.assert_running();
            if polls == 3 {
                let captured = frames.lock().unwrap();
                assert!(captured.len() >= 3, "redraws must precede task completion");
                assert_ne!(
                    captured[1], captured[0],
                    "the first typed key must alter the next frame before any Tick"
                );
            }
            polls += 1;
            clock.set(clock.get() + Duration::from_millis(125));
            events
                .next()
                .ok_or_else(|| color_eyre::eyre::eyre!("loop missed /exit"))
        },
        || clock.get(),
    )
    .unwrap();
    assert_eq!(polls, 10);
    assert_ne!(
        fixture.app.sheen_offset, original_sheen,
        "real deadline must deliver Tick while worker waits"
    );
    assert!(frames
        .lock()
        .unwrap()
        .iter()
        .any(|frame| frame.contains("/exit")));
    assert!(matches!(fixture.app.phase, AppPhase::Quit));
    assert!(fixture.app.tasks.is_empty());
    task.assert_reclaimed();
}

#[test]
fn rf211_progress_and_resize_do_not_starve_deadline_or_reset_idle_time() {
    let _lock = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new();
    let service = Arc::clone(&fixture.app.vault_service);
    let account = service.get_current_account().unwrap();
    let session = service.capture_session(&account).unwrap();
    let (requests, mut receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
    let (reported, reports) = mpsc::channel();
    let (dropped, observed) = mpsc::channel();
    fixture
        .app
        .tasks
        .spawn(session, move |context| async move {
            let _resource = Resource(dropped);
            let mut current = 0;
            while receiver.recv().await.is_some() {
                let mut accepted = 0;
                for _ in 0..96 {
                    current += 1;
                    accepted += usize::from(context.report_progress(current, Some(1000)));
                }
                let _ = reported.send(accepted);
            }
            Ok(TaskOutput::Message("stale synthetic body".to_string()))
        })
        .unwrap();
    let frames = Arc::new(Mutex::new(Vec::new()));
    let mut terminal = Terminal::new(ObservedBackend::new(Arc::clone(&frames), None)).unwrap();
    let base = Instant::now();
    fixture.app.last_activity = base;
    fixture.app.auto_lock_duration = Duration::from_millis(750);
    fixture.app.command_input.set_value("/exit".to_string());
    let clock = ClockCell::new(base);
    let mut polls = 0;
    let mut accepted = Vec::new();
    run_loop_with(
        &mut terminal,
        &mut fixture.app,
        |_| {
            polls += 1;
            if polls <= 3 {
                requests.send(()).unwrap();
                accepted.push(
                    reports
                        .recv_timeout(WAIT)
                        .expect("progress producer must report without blocking runtime"),
                );
                clock.set(clock.get() + Duration::from_millis(250));
                // poll_event 将 Resize 映射为 None；持续就绪的 Resize 不依赖 poll 超时。
                Ok(None)
            } else {
                assert_eq!(
                    polls, 4,
                    "loop must remain responsive under progress backlog"
                );
                assert!(
                    !service.is_unlocked(),
                    "deadline must lock before the next key"
                );
                assert!(frames.lock().unwrap().len() >= 4);
                Ok(key(KeyCode::Enter))
            }
        },
        || clock.get(),
    )
    .unwrap();
    assert_eq!(accepted[0], 64, "real bounded progress queue must fill");
    assert!(
        accepted[1] > 0 && accepted[1] <= 32,
        "one loop turn must consume a bounded batch"
    );
    assert!(fixture.app.tasks.is_empty());
    assert!(fixture.app.info_message.as_deref() != Some("stale synthetic body"));
    observed
        .recv_timeout(WAIT)
        .expect("stale task must be actually reclaimed");
}

#[test]
fn rf211_exit_command_and_confirmation_reclaim_actual_pending_future() {
    let _lock = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for confirm in [false, true] {
        let mut fixture = Fixture::new();
        let task = PendingTask::start(&mut fixture.app);
        let mut terminal = Terminal::new(TestBackend::new(110, 26)).unwrap();
        let mut events = if confirm {
            // 先否决一次：已消费的 prompt 按键不能被当成退出。
            vec![
                key(KeyCode::Esc),
                key(KeyCode::Char('n')),
                key(KeyCode::Esc),
                key(KeyCode::Char('y')),
            ]
        } else {
            fixture.app.command_input.set_value("/exit".to_string());
            vec![key(KeyCode::Enter)]
        }
        .into_iter();
        run_loop_with(
            &mut terminal,
            &mut fixture.app,
            |_| {
                task.assert_running();
                events
                    .next()
                    .ok_or_else(|| color_eyre::eyre::eyre!("loop missed explicit exit"))
            },
            Instant::now,
        )
        .unwrap();
        assert!(matches!(fixture.app.phase, AppPhase::Quit));
        assert!(
            events.next().is_none(),
            "negative confirmation must not prematurely exit"
        );
        assert!(fixture.app.tasks.is_empty());
        task.assert_reclaimed();
    }
}

#[test]
fn rf211_poll_and_backend_draw_errors_reclaim_tasks_and_preserve_primary_error() {
    let _lock = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for draw_failure in [false, true] {
        let mut fixture = Fixture::new();
        let task = PendingTask::start(&mut fixture.app);
        let frames = Arc::new(Mutex::new(Vec::new()));
        let mut terminal =
            Terminal::new(ObservedBackend::new(frames, draw_failure.then_some(1))).unwrap();
        let error = run_loop_with(
            &mut terminal,
            &mut fixture.app,
            |_| Err(color_eyre::eyre::eyre!("RF211 synthetic poll failure")),
            Instant::now,
        )
        .unwrap_err();
        assert!(error.to_string().contains(if draw_failure {
            "synthetic draw failure"
        } else {
            "synthetic poll failure"
        }));
        assert!(fixture.app.tasks.is_empty());
        task.assert_reclaimed();
    }
}

#[test]
fn rf211_terminal_restore_guard_covers_early_return_and_runs_only_once() {
    let restored = ClockCell::new(0);
    let fail_initialization = || -> color_eyre::Result<()> {
        let _guard = TerminalRestoreGuard::new(|| {
            restored.set(restored.get() + 1);
            Ok(())
        });
        Err(color_eyre::eyre::eyre!("synthetic initialization failure"))
    };
    let early_error = fail_initialization();
    assert!(early_error
        .unwrap_err()
        .to_string()
        .contains("initialization failure"));
    assert_eq!(restored.get(), 1);
    {
        let mut guard = TerminalRestoreGuard::new(|| {
            restored.set(restored.get() + 1);
            Err(color_eyre::eyre::eyre!("synthetic restore failure"))
        });
        assert!(guard.restore().is_err());
    }
    assert_eq!(
        restored.get(),
        2,
        "explicit best-effort restore is not repeated by Drop"
    );
}
#[test]
fn rf211_terminal_restore_attempts_every_step_and_preserves_first_error() {
    let mut attempted = Vec::new();
    let error = restore_terminal_with(|step| {
        attempted.push(step);
        match step {
            RestoreStep::RawMode => Err(color_eyre::eyre::eyre!("first synthetic restore failure")),
            RestoreStep::AlternateScreen => {
                Err(color_eyre::eyre::eyre!("later synthetic restore failure"))
            }
            _ => Ok(()),
        }
    })
    .unwrap_err();
    assert_eq!(
        attempted,
        vec![
            RestoreStep::RawMode,
            RestoreStep::MouseCapture,
            RestoreStep::AlternateScreen,
            RestoreStep::Cursor
        ]
    );
    assert_eq!(error.to_string(), "first synthetic restore failure");
}
