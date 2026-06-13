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
