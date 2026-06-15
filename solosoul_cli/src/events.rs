//! 键盘事件映射与分发。

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CEvent, KeyEvent};

/// CLI 内部事件。
#[derive(Debug, Clone)]
pub enum Event {
    /// 键盘事件
    Key(KeyEvent),
    /// 定时 tick，用于空闲检测与自动锁定
    Tick,
}

/// 在指定超时时间内轮询事件。
/// 超时未收到事件时返回 `Event::Tick`。
pub fn poll_event(timeout: Duration) -> Result<Option<Event>> {
    if crossterm::event::poll(timeout)? {
        match crossterm::event::read()? {
            CEvent::Key(key) => Ok(Some(Event::Key(key))),
            // 忽略鼠标、窗口大小改变等事件；窗口大小改变由 ratatui 自动处理
            _ => Ok(None),
        }
    } else {
        Ok(Some(Event::Tick))
    }
}
