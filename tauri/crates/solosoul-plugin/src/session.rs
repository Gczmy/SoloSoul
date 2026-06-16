//! 插件会话管理
//!
//! 每个插件运行实例都关联一个有时效性的会话，过期后自动清理。

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 插件会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSession {
    pub id: String,
    pub plugin_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// 返回给前端的会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSessionInfo {
    pub session_id: String,
    pub plugin_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

impl From<PluginSession> for PluginSessionInfo {
    fn from(s: PluginSession) -> Self {
        Self {
            session_id: s.id,
            plugin_id: s.plugin_id,
            created_at: s.created_at,
            expires_at: s.expires_at,
        }
    }
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
