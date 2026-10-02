//! RF-303：聊天命令、上下文选择和流事件的实际 serde 契约。
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStreamPayload {
    pub account_id: String,
    pub session_generation: u64,
    pub conversation_id: String,
    pub request_id: String,
    pub chunk: String,
    pub is_done: bool,
    pub error: Option<String>,
    /// 新 Host 的安全错误包；旧 error 字段仅保留机器码/持久化前缀。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<crate::commands::error::BackendError>,
}

/// A single chunk returned to the frontend for context injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideChunk {
    pub guide_id: String,
    pub guide_title: String,
    pub chunk_text: String,
    pub similarity: f32,
}

/// 选择标识不携带 Vault 字段值；缺失选择由入口按 None 处理。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ChatContextSelection {
    #[default]
    None,
    PublicProfile {
        #[serde(rename = "objectIds")]
        object_ids: Vec<String>,
        language: String,
        #[serde(rename = "guideChunks")]
        guide_chunks: Vec<GuideChunk>,
    },
}

#[cfg(test)]
mod tests;
