//! 插件审计日志
//!
//! 审计日志采用同步 `std::sync::Mutex`，避免在 Host Function 中引入 async 生命周期问题。

use super::{PluginAuditAction, PluginAuditEntry};
use std::sync::Mutex;

/// 同步审计日志器
pub struct PluginAuditLogger {
    entries: Mutex<Vec<PluginAuditEntry>>,
}

impl Default for PluginAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginAuditLogger {
    /// 创建新的审计日志器
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    /// 记录一条审计日志
    pub fn log(
        &self,
        plugin_id: impl Into<String>,
        session_id: Option<impl Into<String>>,
        action: PluginAuditAction,
    ) {
        let entry = PluginAuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            plugin_id: plugin_id.into(),
            session_id: session_id.map(Into::into),
            action,
        };
        if let Ok(mut guard) = self.entries.lock() {
            guard.push(entry);
        }
    }

    /// 获取最近的 `limit` 条审计日志
    pub fn recent(&self, limit: usize) -> Vec<PluginAuditEntry> {
        let guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        guard.iter().rev().take(limit).cloned().collect()
    }
}
