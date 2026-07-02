//! 插件会话管理
//!
//! 每个插件运行实例都关联一个有时效性的会话，过期后自动清理。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 插件会话
///
/// 序列化为 camelCase 供前端使用，前端字段为 `sessionId`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSession {
    #[serde(rename = "sessionId")]
    pub id: String,
    pub plugin_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// 会话管理器
pub struct PluginSessionManager {
    sessions: Mutex<Vec<PluginSession>>,
}

impl Default for PluginSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginSessionManager {
    /// 创建会话管理器
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
        }
    }

    /// 创建新会话
    pub fn create(&self, plugin_id: &str, ttl_seconds: u64) -> PluginSession {
        let now = now_millis();
        let session = PluginSession {
            id: uuid::Uuid::new_v4().to_string(),
            plugin_id: plugin_id.to_string(),
            created_at: now,
            expires_at: now + Duration::from_secs(ttl_seconds).as_millis() as i64,
        };
        if let Ok(mut guard) = self.sessions.lock() {
            guard.push(session.clone());
        }
        session
    }

    /// 列出所有未过期的会话
    pub fn list_active(&self) -> Vec<PluginSession> {
        self.remove_expired();
        let guard = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }

    /// 清理过期会话
    pub fn remove_expired(&self) {
        let now = now_millis();
        if let Ok(mut guard) = self.sessions.lock() {
            guard.retain(|s| s.expires_at > now);
        }
    }
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_create_session() {
        let mgr = PluginSessionManager::new();
        let session = mgr.create("my_plugin", 3600);
        assert_eq!(session.plugin_id, "my_plugin");
        assert!(session.expires_at > session.created_at);
        assert!(!session.id.is_empty());
    }

    #[test]
    fn test_session_ttl() {
        let mgr = PluginSessionManager::new();
        let session = mgr.create("p1", 60);
        let diff = session.expires_at - session.created_at;
        // TTL = 60s = 60000ms
        assert!(
            (diff - 60_000).abs() < 100,
            "Expected ~60000ms diff, got {}",
            diff
        );
    }

    #[test]
    fn test_list_active_sessions() {
        let mgr = PluginSessionManager::new();
        mgr.create("p1", 3600);
        mgr.create("p2", 3600);
        let active = mgr.list_active();
        assert_eq!(active.len(), 2);
    }

    #[test]
    fn test_remove_expired_sessions() {
        let mgr = PluginSessionManager::new();
        // Create a session with very short TTL
        mgr.create("p1", 0); // 0s = expires immediately
        thread::sleep(Duration::from_millis(10));
        let active = mgr.list_active();
        assert!(
            active.is_empty(),
            "Expected no active sessions, got {}",
            active.len()
        );
    }

    #[test]
    fn test_list_active_only_returns_valid_sessions() {
        let mgr = PluginSessionManager::new();
        mgr.create("p1", 3600); // valid
        mgr.create("p2", 0); // expires immediately
        thread::sleep(Duration::from_millis(10));
        let active = mgr.list_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].plugin_id, "p1");
    }

    #[test]
    fn test_session_serialization() {
        let session = PluginSession {
            id: "s1".to_string(),
            plugin_id: "p1".to_string(),
            created_at: 1000,
            expires_at: 2000,
        };
        // 验证 JSON 序列化使用 sessionId (camelCase)
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("\"sessionId\""));
    }
}
