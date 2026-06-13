//! 插件授权同意管理器
//!
//! 前端通过 `plugin_consent_response` 响应用户授权，Host Function 阻塞等待一次性通道结果。

use std::collections::HashMap;
use tokio::sync::{oneshot, Mutex};

/// 授权管理器
#[derive(Debug, Default)]
pub struct ConsentManager {
    pending: Mutex<HashMap<String, oneshot::Sender<Option<String>>>>,
}

impl ConsentManager {
    /// 创建新的授权管理器
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// 请求一次授权，返回接收端
    pub async fn request_consent(&self, request_id: &str) -> oneshot::Receiver<Option<String>> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.pending.lock().await;
        pending.insert(request_id.to_string(), tx);
        rx
    }

    /// 响应授权请求，发送值并移除条目
    pub async fn respond(&self, request_id: &str, value: Option<String>) -> Result<(), String> {
        let tx = {
            let mut pending = self.pending.lock().await;
            pending.remove(request_id)
        };
        if let Some(tx) = tx {
            tx.send(value).map_err(|_| "授权响应通道已关闭".to_string())
        } else {
            Err(format!("未找到授权请求: {}", request_id))
        }
    }
}
