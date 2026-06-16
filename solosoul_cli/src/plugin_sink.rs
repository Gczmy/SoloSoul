//! Terminal-based PluginEventSink implementation for CLI.

use solosoul_plugin::{PluginEvent, PluginEventSink};

/// 终端插件事件接收器（no-op 实现）。
///
/// 插件运行结果通过 PluginManager::run() 的返回值获取，
/// 此 sink 仅满足 trait 约束，不缓冲事件。
pub struct TerminalPluginSink;

impl PluginEventSink for TerminalPluginSink {
    fn send(&self, _event: PluginEvent) -> Result<(), String> {
        Ok(())
    }
}
