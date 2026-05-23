//! Plugin Session Manager — 活跃 Session 跟踪与撤销

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

/// 会话信息
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub plugin_id: String,
    pub plugin_name: String,
    /// 开始时间（Unix 时间戳，秒）
    pub started_at_secs: i64,
    /// 过期时间（Unix 时间戳，秒）
    pub expires_at_secs: i64,
}

/// 插件会话管理器
///
/// 负责跟踪所有活跃 Session，支持：
/// - 查询当前运行中的插件列表
/// - 按 plugin_id 撤销 Session
/// - 自动清理过期 Session
#[derive(Debug, Clone)]
pub struct PluginSessionManager {
    active: Arc<Mutex<HashMap<String, SessionInfo>>>,
}

impl PluginSessionManager {
    pub fn new() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 注册一个新的活跃 Session
    pub fn register(
        &self,
        plugin_id: &str,
        plugin_name: &str,
        session_id: &str,
        ttl_seconds: u64,
    ) {
        let mut guard = self.active.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        guard.insert(
            plugin_id.to_string(),
            SessionInfo {
                session_id: session_id.to_string(),
                plugin_id: plugin_id.to_string(),
                plugin_name: plugin_name.to_string(),
                started_at_secs: now,
                expires_at_secs: now + ttl_seconds as i64,
            },
        );
    }

    /// 列出所有活跃 Session
    pub fn list_active(&self) -> Vec<SessionInfo> {
        self.cleanup_expired();
        let guard = self.active.lock().unwrap();
        guard.values().cloned().collect()
    }

    /// 检查指定插件是否正在运行
    pub fn is_running(&self, plugin_id: &str) -> bool {
        self.cleanup_expired();
        let guard = self.active.lock().unwrap();
        guard.contains_key(plugin_id)
    }

    /// 撤销指定插件的所有活跃 Session
    ///
    /// 返回被撤销的 Session ID（如果有）
    pub fn revoke(&self, plugin_id: &str) -> Option<String> {
        let mut guard = self.active.lock().unwrap();
        guard.remove(plugin_id).map(|s| s.session_id)
    }

    /// 撤销所有活跃 Session（用于全局锁定或退出）
    pub fn revoke_all(&self) {
        let mut guard = self.active.lock().unwrap();
        guard.clear();
    }

    /// 清理已过期的 Session
    fn cleanup_expired(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let mut guard = self.active.lock().unwrap();
        guard.retain(|_, info| info.expires_at_secs > now);
    }
}

impl Default for PluginSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_lifecycle() {
        let manager = PluginSessionManager::new();

        assert!(!manager.is_running("com.test.plugin"));

        manager.register("com.test.plugin", "Test", "session-1", 300);
        assert!(manager.is_running("com.test.plugin"));

        let active = manager.list_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].plugin_id, "com.test.plugin");

        let revoked = manager.revoke("com.test.plugin");
        assert_eq!(revoked, Some("session-1".to_string()));
        assert!(!manager.is_running("com.test.plugin"));
    }

    #[test]
    fn test_cleanup_expired() {
        let manager = PluginSessionManager::new();

        // 注册一个 0 秒 TTL 的 Session（立即过期）
        manager.register("com.expired.plugin", "Expired", "session-2", 0);

        // 稍微等待确保过期
        std::thread::sleep(Duration::from_millis(10));

        assert!(!manager.is_running("com.expired.plugin"));
        assert!(manager.list_active().is_empty());
    }
}
