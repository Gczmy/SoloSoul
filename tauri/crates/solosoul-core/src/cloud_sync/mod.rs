//! Cloud Sync 抽象层 (Phase 2 · 云同步)。
//!
//! 提供统一的 `CloudConnector` trait，官方实现 `WebDavConnector` 覆盖
//! 坚果云 / Nextcloud / Alist / 自建 WebDAV。OneDrive Graph API 另作实现。
//!
//! 设计原则：
//! - 仅在 Vault 解锁态可用（凭证入 Vault 加密存储，主密钥派生）。
//! - 上传内容**仅限** `.solosoul` 加密快照包（零知识承诺）。
//! - 分片上传（RFC 4918 `PUT` + `Content-Range`），支持断点续传。
//! - 云端布局：`/SoloSoul/{account_id}/snapshots/{device_id}/{hlc}.solosoul`
//!   + 索引文件 `/SoloSoul/{account_id}/latest.json`。

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod webdav;

use webdav::WebDavConnector;

/// 云端同步对象元数据（用于列表/索引）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudObjectMeta {
    pub path: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub etag: Option<String>,
}

/// 云同步配置（存入 Vault 加密字段，仅解锁态可见）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncConfig {
    /// 连接器类型：`webdav` / `onedrive` / `baidu` ...
    pub connector_type: String,
    /// 连接器特定配置（JSON 序列化），反序列化为对应 ConnectorConfig 子类型。
    pub config_json: serde_json::Value,
    /// 自动同步开关。
    pub enabled: bool,
    /// 同步频率秒数（≥ 60）。
    pub interval_secs: u64,
    /// 仅 Wi-Fi 同步（移动端）。
    pub wifi_only: bool,
    /// 云端保留策略：最近 N 份 + 每日/周/月各留一份（GFS）。
    pub retention: RetentionPolicy,
    /// 上次同步时间（用于增量判断）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<DateTime<Utc>>,
}

/// 保留策略（Grandfather-Father-Son 简化版）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// 最近 N 份全量保留。
    pub recent_full: usize,
    /// 是否保留每日快照（0 点最近一份）。
    pub daily: bool,
    /// 是否保留周快照（周一最近一份）。
    pub weekly: bool,
    /// 是否保留月快照（1 号最近一份）。
    pub monthly: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            recent_full: 10,
            daily: true,
            weekly: true,
            monthly: true,
        }
    }
}

impl Default for CloudSyncConfig {
    fn default() -> Self {
        Self {
            connector_type: "webdav".to_string(),
            config_json: serde_json::json!({}),
            enabled: false,
            interval_secs: 3600,
            wifi_only: true,
            retention: RetentionPolicy::default(),
            last_sync_at: None,
        }
    }
}

/// WebDAV 专用配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebDavConfig {
    /// WebDAV 服务器基础 URL（如 `https://dav.jianguoyun.com/dav/`）。
    pub base_url: String,
    /// 用户名（坚果云用邮箱，Nextcloud 用用户名）。
    pub username: String,
    /// 密码 / App Token（入 Vault 前由前端明文传入，后端加密存储）。
    pub password: String,
    /// 云端根路径前缀（默认 `/SoloSoul/`）。
    #[serde(default = "default_root_prefix")]
    pub root_prefix: String,
}

fn default_root_prefix() -> String {
    "/SoloSoul/".to_string()
}

/// 云同步错误。
#[derive(Debug, Error)]
pub enum CloudSyncError {
    #[error("配置缺失：{0}")]
    ConfigMissing(String),
    #[error("连接失败：{0}")]
    ConnectFailed(String),
    #[error("认证失败：{0}")]
    AuthFailed(String),
    #[error("上传失败：{0}")]
    UploadFailed(String),
    #[error("下载失败：{0}")]
    DownloadFailed(String),
    #[error("列表失败：{0}")]
    ListFailed(String),
    #[error("删除失败：{0}")]
    DeleteFailed(String),
    #[error("路径不存在：{0}")]
    NotFound(String),
    #[error("冲突：{0}")]
    Conflict(String),
    #[error("内部错误：{0}")]
    Internal(String),
    #[error("IO 错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP 错误：{0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON 错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("URL 解析错误：{0}")]
    UrlParse(#[from] url::ParseError),
}

/// 统一异步结果类型。
pub type CloudResult<T> = Result<T, CloudSyncError>;

/// 统一异步结果类型。
///
/// 所有方法在 Vault 解锁态调用；实现内部应处理 token 刷新（若适用）。
#[async_trait]
pub trait CloudConnector: Send + Sync {
    /// 连接器类型标识（`"webdav"` / `"onedrive"` 等）。
    fn connector_type(&self) -> &'static str;

    /// 初始化/校验连接（Settings「连接测试」按钮调用）。
    async fn test_connection(&self) -> CloudResult<()>;

    /// 确保远程目录存在（自动创建父目录链，MKCOL 递归）。
    async fn ensure_dir(&self, remote_path: &str) -> CloudResult<()>;

    /// 上传文件（单次流式 PUT，请求体边读边发，内存恒定）。
    ///
    /// `reader` 所有权被接管（`Pin<Box<dyn AsyncRead + Send>>`）；实现内部
    /// 经 ReaderStream 流式发送，任意大小文件均适用。
    /// 返回最终远程路径与 ETag。
    async fn upload(
        &self,
        remote_path: &str,
        reader: Pin<Box<dyn tokio::io::AsyncRead + Send>>,
        total_size: u64,
    ) -> CloudResult<(String, String)>;

    /// 下载文件到 writer。
    async fn download(
        &self,
        remote_path: &str,
        writer: Pin<&mut (dyn tokio::io::AsyncWrite + Send + Unpin)>,
    ) -> CloudResult<u64>;

    /// 列出目录下对象（非递归，仅一级）。
    async fn list(&self, remote_path: &str) -> CloudResult<Vec<CloudObjectMeta>>;

    /// 删除远程文件/目录。
    async fn delete(&self, remote_path: &str) -> CloudResult<()>;

    /// 获取单文件元数据（HEAD 请求）。
    async fn head(&self, remote_path: &str) -> CloudResult<CloudObjectMeta>;
}

/// 连接器工厂（运行时根据配置类型创建实例）。
pub fn create_connector(config: &CloudSyncConfig) -> CloudResult<Arc<dyn CloudConnector>> {
    match config.connector_type.as_str() {
        "webdav" => {
            let cfg: WebDavConfig = serde_json::from_value(config.config_json.clone())
                .map_err(|e| CloudSyncError::ConfigMissing(format!("WebDAV 配置解析失败: {e}")))?;
            Ok(Arc::new(WebDavConnector::new(cfg)))
        }
        other => Err(CloudSyncError::ConfigMissing(format!(
            "未知连接器类型: {other}，当前仅支持 webdav"
        ))),
    }
}

/// 从本地快照文件路径推导云端远程路径。
///
/// 布局：`{root_prefix}{account_id}/snapshots/{device_id}/{hlc}.solosoul`
pub fn build_snapshot_remote_path(
    root_prefix: &str,
    account_id: &str,
    device_id: &str,
    hlc: &str,
) -> String {
    format!("{}/{}/snapshots/{}/{}.solosoul", root_prefix.trim_end_matches('/'), account_id, device_id, hlc)
}

/// 云端索引文件路径：`{root_prefix}{account_id}/latest.json`
pub fn build_latest_index_path(root_prefix: &str, account_id: &str) -> String {
    format!("{}{}/latest.json", root_prefix.trim_end_matches('/'), account_id)
}

/// latest.json 索引结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestIndex {
    /// 设备 ID -> 最新快照元数据
    pub devices: std::collections::HashMap<String, DeviceSnapshotMeta>,
    /// 索引更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshotMeta {
    pub device_id: String,
    pub device_name: Option<String>,
    pub hlc: String,
    pub remote_path: String,
    pub size: u64,
    pub etag: String,
    pub uploaded_at: DateTime<Utc>,
}

impl Default for LatestIndex {
    fn default() -> Self {
        Self {
            devices: std::collections::HashMap::new(),
            updated_at: Utc::now(),
        }
    }
}