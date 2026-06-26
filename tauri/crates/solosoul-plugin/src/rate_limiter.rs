//! 插件频率限制器
//!
//! 按插件 + 动作维度统计最近 60 秒调用次数。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 频率限制器内部状态类型别名
pub type RateLimiterMap = HashMap<String, HashMap<String, (Instant, u32)>>;

/// 简单的固定窗口频率限制器
#[derive(Debug)]
pub struct RateLimiter {
    max_per_minute: u32,
    state: Mutex<RateLimiterMap>,
}

impl RateLimiter {
    /// 创建频率限制器
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            max_per_minute,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// 检查是否允许本次调用
    ///
    /// 返回 `true` 表示未超限，同时更新计数；`false` 表示已触发限流。
    pub fn check(&self, plugin_id: &str, action_key: &str) -> bool {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let plugin_map = state.entry(plugin_id.to_string()).or_default();
        let now = Instant::now();
        let (last, count) = plugin_map.entry(action_key.to_string()).or_insert((now, 0));

        if now.duration_since(*last) > Duration::from_secs(60) {
            *last = now;
            *count = 1;
            true
        } else if *count < self.max_per_minute {
            *count += 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rate_limit_allows_within_limit() {
        let limiter = RateLimiter::new(5);
        for _ in 0..5 {
            assert!(limiter.check("plugin_a", "action_x"));
        }
    }

    #[test]
    fn test_rate_limit_exceeds_max() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check("p1", "a1"));
        assert!(limiter.check("p1", "a1"));
        assert!(limiter.check("p1", "a1"));
        assert!(!limiter.check("p1", "a1")); // 第 4 次应被限流
    }

    #[test]
    fn test_different_actions_independent() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("p1", "action_a"));
        assert!(limiter.check("p1", "action_a"));
        // action_b 应仍可用（独立计数）
        assert!(limiter.check("p1", "action_b"));
        assert!(limiter.check("p1", "action_b"));
        // action_a 应已被限流
        assert!(!limiter.check("p1", "action_a"));
    }

    #[test]
    fn test_different_plugins_independent() {
        let limiter = RateLimiter::new(1);
        assert!(limiter.check("plugin_a", "read"));
        // plugin_b 的 read 应仍可用
        assert!(limiter.check("plugin_b", "read"));
        // plugin_a 的 read 应已被限流
        assert!(!limiter.check("plugin_a", "read"));
    }

    #[test]
    fn test_rate_limit_per_minute_allows_same_plugin() {
        let limiter = RateLimiter::new(10);
        for i in 0..10 {
            assert!(limiter.check("p1", &format!("action_{}", i)));
        }
        // 同一插件但不同动作不应限流
        assert!(limiter.check("p1", "action_new"));
    }

    #[test]
    fn test_different_plugins_and_actions() {
        let limiter = RateLimiter::new(2);
        assert!(limiter.check("p1", "read"));
        assert!(limiter.check("p1", "read"));
        assert!(!limiter.check("p1", "read"));
        // p2 read 独立
        assert!(limiter.check("p2", "read"));
        assert!(limiter.check("p2", "read"));
        assert!(!limiter.check("p2", "read"));
        // p1 write 独立
        assert!(limiter.check("p1", "write"));
    }

    #[test]
    fn test_zero_max_denies_all() {
        let limiter = RateLimiter::new(0);
        assert!(!limiter.check("p1", "a1"));
        assert!(!limiter.check("p1", "a1"));
    }
}
