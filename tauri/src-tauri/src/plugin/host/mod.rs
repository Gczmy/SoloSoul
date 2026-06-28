//! 插件 Host Functions
//!
//! 本模块将 SoloSoul 核心能力通过 `env` 模块暴露给 WebAssembly 插件。
//! ABI 与 `SoloSoul_plugin_market/SDK/rust` 保持一致。

use super::{
    ConsentManager, FieldResolver, PluginAuditAction, PluginAuditLogger, PluginError, PluginEvent,
    PluginLogLine, PluginManifest, PluginResultPayload, RateLimiter,
};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
pub(crate) use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
pub(crate) use std::time::Duration;
use tauri::ipc::Channel;

/// 插件单次 `solosoul_sleep` 最大允许时长（毫秒）。
const MAX_PLUGIN_SLEEP_MS: u64 = 1_000;
/// 插件通过 Host 读取字符串的最大字节数（64 KiB）。
const MAX_PLUGIN_READ_LEN: usize = 64 * 1024;
pub(crate) use tokio::sync::oneshot;
pub(crate) use url::Url;

/// Host Function 错误码（与 SDK `solosoul_plugin_sdk::PluginError` 保持一致）
#[allow(dead_code)]
mod code {
    pub const SUCCESS: i32 = 0;
    pub const PERMISSION_DENIED: i32 = -1;
    pub const USER_DENIED: i32 = -2;
    pub const TTL_EXPIRED: i32 = -3;
    pub const BUFFER_TOO_SMALL: i32 = -4;
    pub const INVALID_FIELD: i32 = -5;
    pub const NETWORK_TIMEOUT: i32 = -6;
    pub const VAULT_LOCKED: i32 = -7;
    pub const RATE_LIMITED: i32 = -8;
    pub const NOT_IMPLEMENTED: i32 = -9;
    pub const DOMAIN_NOT_ALLOWED: i32 = -10;
    pub const INVALID_ARGUMENT: i32 = -11;
    pub const WASM_TRAP: i32 = -12;
    pub const FILE_NOT_FOUND: i32 = -13;
    pub const FILE_TOO_LARGE: i32 = -14;
    pub const PROCESSING_FAILED: i32 = -15;
    /// 异步 HTTP 请求仍在进行中（非错误，仅用于轮询）
    pub const HTTP_PENDING: i32 = 1;
}

/// 传递给 Wasm Store 的状态，包含 WASI 上下文与自定义 Host 数据
pub struct SoloHostState {
    pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
    pub host: SoloHostFunctions,
}

/// 异步 HTTP 请求结果
#[derive(Debug, Clone)]
pub(crate) struct HttpResult {
    pub status: u16,
    pub body: String,
    pub error_code: Option<i32>,
}

/// 异步 HTTP 请求句柄状态
#[derive(Debug)]
pub(crate) enum HttpHandleState {
    Running {
        rx: oneshot::Receiver<HttpResult>,
        abort: tokio::task::AbortHandle,
    },
    Completed(HttpResult),
}

/// 自定义 Host Functions 数据
#[allow(clippy::module_name_repetitions)]
pub struct SoloHostFunctions {
    pub plugin_id: String,
    pub plugin_name: String,
    pub session_id: String,
    pub manifest: PluginManifest,
    pub params: HashMap<String, String>,
    pub logs: Mutex<Vec<PluginLogLine>>,
    pub results: Mutex<Vec<PluginResultPayload>>,
    pub audit: Arc<PluginAuditLogger>,
    pub rate_limiter: Arc<RateLimiter>,
    pub consent_manager: Arc<ConsentManager>,
    pub field_resolver: Arc<FieldResolver>,
    pub channel: Channel<PluginEvent>,
    pub(crate) http_handles: Arc<Mutex<HashMap<u32, HttpHandleState>>>,
    pub(crate) next_http_handle: AtomicU32,
    /// Shared HTTP client reused across plugin HTTP calls.
    pub(crate) http_client: reqwest::Client,
    /// 插件运行时的临时工作区目录（用于附件复制、水印处理等）。
    pub workspace_dir: Option<PathBuf>,
}

impl Drop for SoloHostFunctions {
    fn drop(&mut self) {
        let mut handles = self.http_handles.lock().unwrap_or_else(|e| e.into_inner());
        for (_, state) in handles.drain() {
            if let HttpHandleState::Running { abort, .. } = state {
                abort.abort();
            }
        }
    }
}

impl SoloHostFunctions {
    /// 创建 Host Functions 数据
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        session_id: impl Into<String>,
        manifest: PluginManifest,
        params: HashMap<String, String>,
        audit: Arc<PluginAuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        consent_manager: Arc<ConsentManager>,
        field_resolver: Arc<FieldResolver>,
        channel: Channel<PluginEvent>,
    ) -> Self {
        Self::new_with_workspace(
            plugin_id,
            plugin_name,
            session_id,
            manifest,
            params,
            audit,
            rate_limiter,
            consent_manager,
            field_resolver,
            channel,
            None,
        )
    }

    /// 创建 Host Functions 数据，并指定临时工作区目录。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_workspace(
        plugin_id: impl Into<String>,
        plugin_name: impl Into<String>,
        session_id: impl Into<String>,
        manifest: PluginManifest,
        params: HashMap<String, String>,
        audit: Arc<PluginAuditLogger>,
        rate_limiter: Arc<RateLimiter>,
        consent_manager: Arc<ConsentManager>,
        field_resolver: Arc<FieldResolver>,
        channel: Channel<PluginEvent>,
        workspace_dir: Option<PathBuf>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            plugin_id: plugin_id.into(),
            plugin_name: plugin_name.into(),
            session_id: session_id.into(),
            manifest,
            params,
            logs: Mutex::new(Vec::new()),
            results: Mutex::new(Vec::new()),
            audit,
            rate_limiter,
            consent_manager,
            field_resolver,
            channel,
            http_handles: Arc::new(Mutex::new(HashMap::new())),
            next_http_handle: AtomicU32::new(1),
            http_client,
            workspace_dir,
        }
    }

    /// 取出运行期间收集的日志
    pub fn take_logs(&self) -> Vec<PluginLogLine> {
        let mut guard = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard)
    }

    /// 取出运行期间收集的结构化结果
    pub fn take_results(&self) -> Vec<PluginResultPayload> {
        let mut guard = self.results.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *guard)
    }
}

// 注册所有 Host Functions 到 linker

// ── Sub-modules ─────────────────────────────────────────────

pub(crate) mod http;
pub(crate) mod memory;
pub mod register;
#[cfg(test)]
pub mod tests;
pub use register::register_host_functions;
