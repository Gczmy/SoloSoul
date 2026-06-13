//! 插件事件
//!
//! 插件运行期间通过 Tauri Channel 向前端发送日志、结果、授权请求与生命周期事件。

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 插件事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEvent {
    pub event_type: String,
    pub json_data: String,
    pub request_id: Option<String>,
    pub plugin_id: Option<String>,
    pub plugin_name: Option<String>,
    pub field_id: Option<String>,
    pub field_label: Option<String>,
    pub sensitivity_level: Option<String>,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn make_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

impl PluginEvent {
    /// 日志事件
    pub fn log(level: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            event_type: "log".to_string(),
            json_data: serde_json::json!({
                "id": make_id(),
                "level": level.into(),
                "message": message.into(),
                "timestamp": now_millis(),
            })
            .to_string(),
            request_id: None,
            plugin_id: None,
            plugin_name: None,
            field_id: None,
            field_label: None,
            sensitivity_level: None,
        }
    }

    /// 结构化结果事件
    pub fn result(json: impl Into<String>) -> Self {
        Self {
            event_type: "result".to_string(),
            json_data: json.into(),
            request_id: None,
            plugin_id: None,
            plugin_name: None,
            field_id: None,
            field_label: None,
            sensitivity_level: None,
        }
    }

    /// 授权请求事件
    #[allow(clippy::too_many_arguments)]
    pub fn consent_request(
        request_id: impl Into<String>,
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        field_id: impl Into<String>,
        field_label: impl Into<String>,
        sensitivity_level: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "consent_request".to_string(),
            json_data: serde_json::json!({}).to_string(),
            request_id: Some(request_id.into()),
            plugin_id: Some(plugin_id.into()),
            plugin_name: Some(plugin_name.into()),
            field_id: Some(field_id.into()),
            field_label: Some(field_label.into()),
            sensitivity_level: Some(sensitivity_level.into()),
        }
    }

    /// 运行完成事件
    pub fn completed(plugin_id: impl Into<String>, exit_code: i32, fuel_consumed: u64) -> Self {
        Self {
            event_type: "completed".to_string(),
            json_data: serde_json::json!({
                "exitCode": exit_code,
                "fuelConsumed": fuel_consumed,
            })
            .to_string(),
            request_id: None,
            plugin_id: Some(plugin_id.into()),
            plugin_name: None,
            field_id: None,
            field_label: None,
            sensitivity_level: None,
        }
    }

    /// 运行错误事件
    pub fn error(plugin_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            event_type: "error".to_string(),
            json_data: serde_json::json!({ "message": message.into() }).to_string(),
            request_id: None,
            plugin_id: Some(plugin_id.into()),
            plugin_name: None,
            field_id: None,
            field_label: None,
            sensitivity_level: None,
        }
    }
}
