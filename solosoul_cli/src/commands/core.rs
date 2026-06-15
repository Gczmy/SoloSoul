//! 核心全局命令（/exit、/back 等）。

use crate::app::{App, AppPhase};

/// 执行 `/exit`：标记退出。
pub fn exit(app: &mut App) {
    app.phase = AppPhase::Quit;
}

/// 执行 `/back`：返回上一屏。
pub fn back(app: &mut App) {
    if let Some(prev) = app.previous_phase.take() {
        app.phase = prev;
    } else {
        app.error_message = Some("没有上一屏可返回".to_string());
    }
}
