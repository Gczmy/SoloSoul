//! 插件 Host Functions
//!
//! 本模块将 SoloSoul 核心能力通过 `env` 模块暴露给 WebAssembly 插件。
//! ABI 与 `SoloSoul_plugin_market/SDK/rust` 保持一致。

use super::{
    ConsentManager, FieldResolver, PluginAuditAction, PluginAuditLogger, PluginError, PluginEvent,
    PluginLogLine, PluginManifest, PluginResultPayload, RateLimiter,
};
use crate::event::PluginEventSink;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 插件单次 `solosoul_sleep` 最大允许时长（毫秒）。
const MAX_PLUGIN_SLEEP_MS: u64 = 1_000;
/// 插件通过 Host 读取字符串的最大字节数（64 KiB）。
const MAX_PLUGIN_READ_LEN: usize = 64 * 1024;
use tokio::sync::oneshot;
use url::Url;
use wasmtime::{Caller, Extern, Linker, Memory};

/// Host Function 错误码（与 SDK `solosoul_plugin_sdk::PluginError` 保持一致）
///
/// 公开导出，供 `src-tauri` 中的插件 Host 模块复用，避免错误码定义重复。
pub mod code {
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
    pub channel: std::sync::Arc<dyn PluginEventSink>,
    pub(crate) http_handles: Arc<Mutex<HashMap<u32, HttpHandleState>>>,
    pub(crate) next_http_handle: AtomicU32,
    /// Shared HTTP client reused across plugin HTTP calls.
    pub(crate) http_client: reqwest::Client,
    /// 插件运行时的临时工作区目录（用于附件复制、水印处理等）。
    pub workspace_dir: Option<std::path::PathBuf>,
}

impl SoloHostFunctions {
    /// 频率限制检查：以当前插件身份对指定操作名做限流。
    ///
    /// WASI 宿主函数共用同一检查模式（`rate_limiter.check(&plugin_id, name)`），
    /// 抽成方法避免 7 处重复展开（P223-① 预案微优化收尾）。
    fn check_rate(&self, name: &str) -> bool {
        self.rate_limiter.check(&self.plugin_id, name)
    }
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
        channel: std::sync::Arc<dyn PluginEventSink>,
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
        channel: std::sync::Arc<dyn PluginEventSink>,
        workspace_dir: Option<std::path::PathBuf>,
    ) -> Self {
        // P003: 关闭自动跟随重定向——白名单只校验初始 URL 的 host，reqwest 默认跟随
        // 最多 10 跳会把请求引到白名单外主机（含 localhost/169.254.169.254 等），
        // 开放重定向即可绕过沙箱边界。3xx 作为普通响应返回插件，插件继续请求新域名
        // 时仍会再次经过 is_domain_allowed 校验。
        let http_client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
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

/// 注册水印宿主函数（桌面端）：图片/PDF 共用同一套校验与执行逻辑
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn register_watermark_fn(
    linker: &mut Linker<SoloHostState>,
    func_name: &str,
    label: &'static str,
    apply: fn(&Path, &Path, &solosoul_core::watermark::WatermarkConfig) -> Result<(), String>,
) -> Result<(), PluginError> {
    linker
        .func_wrap(
            "env",
            func_name,
            move |mut caller: Caller<'_, SoloHostState>,
                  input_path_ptr: i32,
                  input_path_len: i32,
                  output_path_ptr: i32,
                  output_path_len: i32,
                  config_json_ptr: i32,
                  config_json_len: i32|
                  -> i32 {
                let input_path =
                    match read_required_string(&mut caller, input_path_ptr, input_path_len) {
                        Some(s) => PathBuf::from(s),
                        None => return code::INVALID_ARGUMENT,
                    };
                let output_path =
                    match read_required_string(&mut caller, output_path_ptr, output_path_len) {
                        Some(s) => PathBuf::from(s),
                        None => return code::INVALID_ARGUMENT,
                    };
                let config_json =
                    match read_required_string(&mut caller, config_json_ptr, config_json_len) {
                        Some(s) => s,
                        None => return code::INVALID_ARGUMENT,
                    };

                if !is_under_workspace(&caller.data().host, &input_path)
                    || !is_under_workspace(&caller.data().host, &output_path)
                {
                    return code::PERMISSION_DENIED;
                }

                let config =
                    match solosoul_core::watermark::WatermarkConfig::from_json(&config_json) {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = caller.data().host.channel.send(PluginEvent::log(
                                "error",
                                format!("{} 配置解析失败: {}", label, e),
                            ));
                            return code::INVALID_ARGUMENT;
                        }
                    };

                match apply(&input_path, &output_path, &config) {
                    Ok(()) => code::SUCCESS,
                    Err(e) => {
                        let _ = caller
                            .data()
                            .host
                            .channel
                            .send(PluginEvent::log("error", format!("{} 失败: {}", label, e)));
                        code::PROCESSING_FAILED
                    }
                }
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    Ok(())
}

/// 注册所有 Host Functions 到 linker
pub fn register_host_functions(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    register_field_access_fns(linker)?;
    register_http_fns(linker)?;
    register_output_fns(linker)?;
    register_watermark_host_fns(linker)?;
    register_interaction_fns(linker)?;
    register_util_fns(linker)?;
    Ok(())
}

/// 字段/数据访问簇：request_field / list_objects / get_data_structure_tree / get_param（P223-① 分簇）
fn register_field_access_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    // solosoul_request_field —— 请求字段
    linker
        .func_wrap("env", "solosoul_request_field", solosoul_request_field_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_list_objects —— 列出指定类型的所有对象（Phase 5，替代 .count）
    linker
        .func_wrap("env", "solosoul_list_objects", solosoul_list_objects_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_get_data_structure_tree —— 数据结构树（元数据）
    linker
        .func_wrap(
            "env",
            "solosoul_get_data_structure_tree",
            solosoul_get_data_structure_tree_impl,
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_get_param —— 获取运行参数
    linker
        .func_wrap("env", "solosoul_get_param", solosoul_get_param_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_list_attachments —— 列出可水印的附件树
    linker
        .func_wrap(
            "env",
            "solosoul_list_attachments",
            solosoul_list_attachments_impl,
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    Ok(())
}
/// solosoul_request_field —— 请求字段
fn solosoul_request_field_impl(
    mut caller: Caller<'_, SoloHostState>,
    field_id_ptr: i32,
    field_id_len: i32,
    out_ptr: i32,
    out_len: i32,
) -> i32 {
    let field_id = match read_string(&mut caller, field_id_ptr, field_id_len) {
        Ok(s) => s,
        Err(_) => return code::INVALID_ARGUMENT,
    };
    let (plugin_id, session_id) = {
        let host = &caller.data().host;
        host.audit.log(
            &host.plugin_id,
            Some(&host.session_id),
            PluginAuditAction::PluginRunStarted,
        );
        if !host.check_rate("request_field") {
            return code::RATE_LIMITED;
        }
        (host.plugin_id.clone(), host.session_id.clone())
    };
    let result = caller.data().host.field_resolver.resolve(&field_id);
    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );
    match result {
        Ok(value) => write_buffer(&mut caller, out_ptr, out_len, &value, -1),
        Err(e) => plugin_error_code(&e),
    }
}

/// solosoul_list_objects —— 列出指定类型的所有对象（Phase 5，替代 .count）
fn solosoul_list_objects_impl(
    mut caller: Caller<'_, SoloHostState>,
    type_id_ptr: i32,
    type_id_len: i32,
    out_ptr: i32,
    out_cap: i32,
) -> i32 {
    let type_id = match read_required_string(&mut caller, type_id_ptr, type_id_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };
    let (plugin_id, session_id) = {
        let host = &caller.data().host;
        if !host.check_rate("list_objects") {
            return code::RATE_LIMITED;
        }
        (host.plugin_id.clone(), host.session_id.clone())
    };
    let result = caller.data().host.field_resolver.list_objects(&type_id);
    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );
    match result {
        Ok(json) => write_buffer(&mut caller, out_ptr, out_cap, &json, -1),
        Err(e) => plugin_error_code(&e),
    }
}

/// solosoul_get_data_structure_tree —— 数据结构树（元数据）
fn solosoul_get_data_structure_tree_impl(
    mut caller: Caller<'_, SoloHostState>,
    out_ptr: i32,
    out_len: i32,
) -> i32 {
    let (plugin_id, session_id) = {
        let host = &caller.data().host;
        if !host.check_rate("get_data_structure_tree") {
            return code::RATE_LIMITED;
        }
        (host.plugin_id.clone(), host.session_id.clone())
    };

    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );

    match caller.data().host.field_resolver.build_structure_tree() {
        Ok(json) => write_buffer(&mut caller, out_ptr, out_len, &json, -1),
        Err(e) => plugin_error_code(&e),
    }
}

/// solosoul_get_param —— 获取运行参数
fn solosoul_get_param_impl(
    mut caller: Caller<'_, SoloHostState>,
    key_ptr: i32,
    key_len: i32,
    out_ptr: i32,
    out_len: i32,
    written_ptr: i32,
) -> i32 {
    let key = match read_string(&mut caller, key_ptr, key_len) {
        Ok(s) => s,
        Err(_) => return code::INVALID_ARGUMENT,
    };
    let value = caller
        .data()
        .host
        .params
        .get(&key)
        .cloned()
        .unwrap_or_default();
    write_buffer(&mut caller, out_ptr, out_len, &value, written_ptr)
}

/// solosoul_list_attachments —— 列出可水印的附件树
fn solosoul_list_attachments_impl(
    mut caller: Caller<'_, SoloHostState>,
    out_ptr: i32,
    out_cap: i32,
) -> i32 {
    let resolver = caller.data().host.field_resolver.clone();
    match resolver.list_attachments() {
        Ok(json) => write_buffer(&mut caller, out_ptr, out_cap, &json, -1),
        Err(e) => plugin_error_code(&e),
    }
}

/// HTTP 簇：http_request / http_poll / http_read / http_close（P223-① 分簇）
fn register_http_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    linker
        .func_wrap("env", "solosoul_http_request", http_request_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_http_poll", http_poll_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_http_read", http_read_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_http_close", http_close_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    Ok(())
}

/// solosoul_http_request —— 发起异步 HTTP 请求（返回句柄）
// WASM host 函数参数与调用约定一一对应，无法合并，故允许 8 参数。
#[allow(clippy::too_many_arguments)]
fn http_request_impl(
    mut caller: Caller<'_, SoloHostState>,
    method_ptr: i32,
    method_len: i32,
    url_ptr: i32,
    url_len: i32,
    body_ptr: i32,
    body_len: i32,
    out_handle_ptr: i32,
) -> i32 {
    let method = match read_required_string(&mut caller, method_ptr, method_len) {
        Some(s) => s.to_uppercase(),
        None => return code::INVALID_ARGUMENT,
    };
    let url = match read_required_string(&mut caller, url_ptr, url_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };
    let body = match read_string(&mut caller, body_ptr, body_len) {
        Ok(s) => s,
        Err(_) => return code::INVALID_ARGUMENT,
    };

    if !matches!(method.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
        return code::INVALID_ARGUMENT;
    }

    let (plugin_id, session_id, audit, handle) = {
        let host = &caller.data().host;
        if !host.check_rate("http_request") {
            return code::RATE_LIMITED;
        }
        if host.manifest.network_policy.block_all_outbound {
            return code::DOMAIN_NOT_ALLOWED;
        }
        let parsed_url = match Url::parse(&url) {
            Ok(u) => u,
            Err(_) => return code::INVALID_ARGUMENT,
        };
        let domain = parsed_url.host_str().unwrap_or("").to_lowercase();
        if domain.is_empty()
            || !is_domain_allowed(&domain, &host.manifest.network_policy.allowed_domains)
        {
            return code::DOMAIN_NOT_ALLOWED;
        }

        let handle = host.next_http_handle.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        let channel = host.channel.clone();
        let client = host.http_client.clone();
        let method_clone = method;
        let url_clone = url;
        let task = tokio::spawn(async move {
            let result = perform_http_async(&client, &method_clone, &url_clone, &body).await;
            if let Err(ref e) = result {
                let _ = channel.send(PluginEvent::log(
                    "error",
                    format!("solosoul_http_request 失败: {}", e),
                ));
            }
            let _ = tx.send(result.unwrap_or_else(|code| HttpResult {
                status: 0,
                body: String::new(),
                error_code: Some(code),
            }));
        });
        let abort = task.abort_handle();

        {
            let mut handles = host.http_handles.lock().unwrap_or_else(|e| e.into_inner());
            handles.insert(handle, HttpHandleState::Running { rx, abort });
        }

        (
            host.plugin_id.clone(),
            host.session_id.clone(),
            host.audit.clone(),
            handle,
        )
    };

    if write_u32(&mut caller, out_handle_ptr, handle) != code::SUCCESS {
        return code::INVALID_ARGUMENT;
    }

    audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );

    code::SUCCESS
}

/// solosoul_http_poll —— 轮询异步 HTTP 请求状态
fn http_poll_impl(
    mut caller: Caller<'_, SoloHostState>,
    handle: i32,
    out_status_ptr: i32,
    out_len_ptr: i32,
) -> i32 {
    if handle < 0 {
        return code::INVALID_ARGUMENT;
    }
    let handle = handle as u32;

    let (result, code_result) = {
        let host = &caller.data().host;
        let mut handles = host.http_handles.lock().unwrap_or_else(|e| e.into_inner());
        let state = match handles.get_mut(&handle) {
            Some(s) => s,
            None => return code::INVALID_ARGUMENT,
        };

        match state {
            HttpHandleState::Running { rx, .. } => match rx.try_recv() {
                Ok(result) => {
                    let code_result = result.error_code.unwrap_or(code::SUCCESS);
                    *state = HttpHandleState::Completed(result.clone());
                    (Some(result), code_result)
                }
                Err(oneshot::error::TryRecvError::Empty) => (None, code::HTTP_PENDING),
                Err(oneshot::error::TryRecvError::Closed) => {
                    let result = HttpResult {
                        status: 0,
                        body: String::new(),
                        error_code: Some(code::NETWORK_TIMEOUT),
                    };
                    *state = HttpHandleState::Completed(result.clone());
                    (Some(result), code::NETWORK_TIMEOUT)
                }
            },
            HttpHandleState::Completed(result) => {
                let code_result = result.error_code.unwrap_or(code::SUCCESS);
                (Some(result.clone()), code_result)
            }
        }
    };

    if let Some(result) = result {
        write_http_poll_result(&mut caller, out_status_ptr, out_len_ptr, &result)
    } else {
        code_result
    }
}

/// solosoul_http_read —— 读取异步 HTTP 响应体
fn http_read_impl(
    mut caller: Caller<'_, SoloHostState>,
    handle: i32,
    out_ptr: i32,
    out_cap: i32,
    written_ptr: i32,
) -> i32 {
    if handle < 0 {
        return code::INVALID_ARGUMENT;
    }
    let handle = handle as u32;

    let result = {
        let host = &caller.data().host;
        let mut handles = host.http_handles.lock().unwrap_or_else(|e| e.into_inner());
        match handles.get_mut(&handle) {
            Some(HttpHandleState::Completed(r)) => r.clone(),
            Some(HttpHandleState::Running { .. }) => return code::HTTP_PENDING,
            _ => return code::INVALID_ARGUMENT,
        }
    };

    if let Some(error_code) = result.error_code {
        return error_code;
    }

    // 截断到 64KB，与同步 post_data 保持一致
    let truncated: String = result.body.chars().take(64 * 1024).collect();
    write_buffer(&mut caller, out_ptr, out_cap, &truncated, written_ptr)
}

/// solosoul_http_close —— 关闭并释放异步 HTTP 句柄
fn http_close_impl(caller: Caller<'_, SoloHostState>, handle: i32) -> i32 {
    if handle < 0 {
        return code::INVALID_ARGUMENT;
    }
    let handle = handle as u32;
    let host = &caller.data().host;
    let mut handles = host.http_handles.lock().unwrap_or_else(|e| e.into_inner());
    match handles.remove(&handle) {
        Some(_) => code::SUCCESS,
        None => code::INVALID_ARGUMENT,
    }
}

/// 输出/附件簇：prepare_attachment_copy / copy_output_file / write_output_file / list_attachments（P223-① 分簇）
fn register_output_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    linker
        .func_wrap(
            "env",
            "solosoul_prepare_attachment_copy",
            prepare_attachment_copy_impl,
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_copy_output_file", copy_output_file_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_write_output_file", write_output_file_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    Ok(())
}

/// solosoul_prepare_attachment_copy —— 将 Vault 附件复制到插件临时工作区
fn prepare_attachment_copy_impl(
    mut caller: Caller<'_, SoloHostState>,
    object_id_ptr: i32,
    object_id_len: i32,
    attachment_id_ptr: i32,
    attachment_id_len: i32,
    out_path_ptr: i32,
    out_path_cap: i32,
) -> i32 {
    let object_id = match read_required_string(&mut caller, object_id_ptr, object_id_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };
    let attachment_id =
        match read_required_string(&mut caller, attachment_id_ptr, attachment_id_len) {
            Some(s) => s,
            None => return code::INVALID_ARGUMENT,
        };

    let workspace = match caller.data().host.workspace_dir.as_ref() {
        Some(d) => d.clone(),
        None => return code::NOT_IMPLEMENTED,
    };

    let resolver = caller.data().host.field_resolver.clone();
    match copy_attachment_to_workspace(&resolver, &workspace, &object_id, &attachment_id) {
        Ok(path) => write_buffer(
            &mut caller,
            out_path_ptr,
            out_path_cap,
            path.to_string_lossy().as_ref(),
            -1,
        ),
        Err(e) => {
            let _ = caller.data().host.channel.send(PluginEvent::log(
                "error",
                format!("prepare_attachment_copy 失败: {}", e),
            ));
            code::PROCESSING_FAILED
        }
    }
}

/// solosoul_copy_output_file —— 将工作区中的已处理文件复制到输出目录
fn copy_output_file_impl(
    mut caller: Caller<'_, SoloHostState>,
    src_path_ptr: i32,
    src_path_len: i32,
    file_name_ptr: i32,
    file_name_len: i32,
    out_path_ptr: i32,
    out_path_cap: i32,
) -> i32 {
    let src_path = match read_required_string(&mut caller, src_path_ptr, src_path_len) {
        Some(s) => PathBuf::from(s),
        None => return code::INVALID_ARGUMENT,
    };
    let file_name = match read_required_string(&mut caller, file_name_ptr, file_name_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };

    if !is_under_workspace(&caller.data().host, &src_path) {
        return code::PERMISSION_DENIED;
    }

    let output_dir = match caller
        .data()
        .host
        .params
        .get("outputDir")
        .filter(|s| !s.is_empty())
    {
        Some(d) => PathBuf::from(d),
        None => return code::INVALID_ARGUMENT,
    };

    match copy_output_file(&src_path, &output_dir, &file_name) {
        Ok(path) => write_buffer(
            &mut caller,
            out_path_ptr,
            out_path_cap,
            path.to_string_lossy().as_ref(),
            -1,
        ),
        Err(e) => {
            let _ = caller.data().host.channel.send(PluginEvent::log(
                "error",
                format!("copy_output_file 失败: {}", e),
            ));
            code::PROCESSING_FAILED
        }
    }
}

/// solosoul_write_output_file —— 将字节写入输出目录
fn write_output_file_impl(
    mut caller: Caller<'_, SoloHostState>,
    file_name_ptr: i32,
    file_name_len: i32,
    bytes_ptr: i32,
    bytes_len: i32,
    out_path_ptr: i32,
    out_path_cap: i32,
) -> i32 {
    let file_name = match read_required_string(&mut caller, file_name_ptr, file_name_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };
    if bytes_len < 0 || bytes_len as usize > 256 * 1024 * 1024 {
        return code::FILE_TOO_LARGE;
    }
    let bytes_len = bytes_len as usize;
    let bytes = match read_bytes(&mut caller, bytes_ptr, bytes_len) {
        Ok(b) => b,
        Err(_) => return code::INVALID_ARGUMENT,
    };

    let output_dir = match caller
        .data()
        .host
        .params
        .get("outputDir")
        .filter(|s| !s.is_empty())
    {
        Some(d) => PathBuf::from(d),
        None => return code::INVALID_ARGUMENT,
    };

    match write_output_file(&output_dir, &file_name, &bytes) {
        Ok(path) => write_buffer(
            &mut caller,
            out_path_ptr,
            out_path_cap,
            path.to_string_lossy().as_ref(),
            -1,
        ),
        Err(e) => {
            let _ = caller.data().host.channel.send(PluginEvent::log(
                "error",
                format!("write_output_file 失败: {}", e),
            ));
            code::PROCESSING_FAILED
        }
    }
}

/// 水印簇：image_watermark / pdf_watermark（P223-① 分簇；命名避开既有 register_watermark_fn）
fn register_watermark_host_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    // solosoul_image_watermark —— 为图片添加水印
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    register_watermark_fn(
        linker,
        "solosoul_image_watermark",
        "image_watermark",
        solosoul_core::watermark::apply_to_image,
    )?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    linker
        .func_wrap(
            "env",
            "solosoul_image_watermark",
            |_caller: Caller<'_, SoloHostState>,
             _input_path_ptr: i32,
             _input_path_len: i32,
             _output_path_ptr: i32,
             _output_path_len: i32,
             _config_json_ptr: i32,
             _config_json_len: i32|
             -> i32 { code::NOT_IMPLEMENTED },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_pdf_watermark —— 为 PDF 添加水印
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    register_watermark_fn(
        linker,
        "solosoul_pdf_watermark",
        "pdf_watermark",
        solosoul_core::watermark::apply_to_pdf,
    )?;

    #[cfg(any(target_os = "android", target_os = "ios"))]
    linker
        .func_wrap(
            "env",
            "solosoul_pdf_watermark",
            |_caller: Caller<'_, SoloHostState>,
             _input_path_ptr: i32,
             _input_path_len: i32,
             _output_path_ptr: i32,
             _output_path_len: i32,
             _config_json_ptr: i32,
             _config_json_len: i32|
             -> i32 { code::NOT_IMPLEMENTED },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    Ok(())
}

/// 交互簇：request_consent / show_dialog / log（P223-① 分簇）
fn register_interaction_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    linker
        .func_wrap("env", "solosoul_request_consent", request_consent_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_show_dialog", show_dialog_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    linker
        .func_wrap("env", "solosoul_log", log_impl)
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
    Ok(())
}

/// solosoul_request_consent —— 请求用户授权（阻塞等待用户响应）
fn request_consent_impl(
    mut caller: Caller<'_, SoloHostState>,
    field_id_ptr: i32,
    field_id_len: i32,
    request_id_ptr: i32,
    request_id_len: i32,
) -> i32 {
    let field_id = read_string(&mut caller, field_id_ptr, field_id_len).unwrap_or_default();
    let request_id = read_string(&mut caller, request_id_ptr, request_id_len).unwrap_or_default();
    if field_id.is_empty() || request_id.is_empty() {
        return code::INVALID_ARGUMENT;
    }

    let (plugin_id, plugin_name, session_id, consent_manager) = {
        let host = &caller.data().host;
        if !host.check_rate("request_consent") {
            return code::RATE_LIMITED;
        }
        (
            host.plugin_id.clone(),
            host.plugin_name.clone(),
            host.session_id.clone(),
            host.consent_manager.clone(),
        )
    };

    // 尝试从 Vault Schema 读取真实字段标签与敏感度；失败时回退到字段 ID 本身
    let (field_label, sensitivity_level) = caller
        .data()
        .host
        .field_resolver
        .field_metadata(&field_id)
        .unwrap_or_else(|_| (field_id.clone(), "sensitive".to_string()));

    let event = PluginEvent::consent_request(
        &request_id,
        &plugin_id,
        &plugin_name,
        &field_id,
        &field_label,
        &sensitivity_level,
    );
    let _ = caller.data().host.channel.send(event);
    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );

    // 阻塞等待用户响应，超时 5 分钟
    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => return code::NOT_IMPLEMENTED,
    };
    let rx = handle.block_on(consent_manager.request_consent(&request_id));

    match handle.block_on(tokio::time::timeout(Duration::from_secs(300), rx)) {
        Ok(Ok(Some(_value))) => {
            caller.data().host.audit.log(
                &plugin_id,
                Some(&session_id),
                PluginAuditAction::ConsentApproved { field_id },
            );
            code::SUCCESS
        }
        Ok(Ok(None)) => {
            caller.data().host.audit.log(
                &plugin_id,
                Some(&session_id),
                PluginAuditAction::ConsentDenied { field_id },
            );
            code::USER_DENIED
        }
        Ok(Err(_)) | Err(_) => {
            caller.data().host.audit.log(
                &plugin_id,
                Some(&session_id),
                PluginAuditAction::ConsentDenied { field_id },
            );
            code::TTL_EXPIRED
        }
    }
}

/// solosoul_show_dialog —— 通用对话框（阻塞等待用户响应）
fn show_dialog_impl(
    mut caller: Caller<'_, SoloHostState>,
    config_ptr: i32,
    config_len: i32,
    out_ptr: i32,
    out_len: i32,
) -> i32 {
    let config = match read_required_string(&mut caller, config_ptr, config_len) {
        Some(s) => s,
        None => return code::INVALID_ARGUMENT,
    };
    if config.len() > 4096 {
        return code::INVALID_ARGUMENT;
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let (plugin_id, plugin_name, session_id, consent_manager) = {
        let host = &caller.data().host;
        if !host.check_rate("show_dialog") {
            return code::RATE_LIMITED;
        }
        (
            host.plugin_id.clone(),
            host.plugin_name.clone(),
            host.session_id.clone(),
            host.consent_manager.clone(),
        )
    };

    let event = PluginEvent::dialog_request(&request_id, &plugin_id, &plugin_name, &config);
    let _ = caller.data().host.channel.send(event);
    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );

    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => return code::NOT_IMPLEMENTED,
    };
    let rx = handle.block_on(consent_manager.request_consent(&request_id));

    match handle.block_on(tokio::time::timeout(Duration::from_secs(300), rx)) {
        Ok(Ok(Some(value))) => write_buffer(&mut caller, out_ptr, out_len, &value, -1),
        Ok(Ok(None)) => code::USER_DENIED,
        Ok(Err(_)) | Err(_) => code::TTL_EXPIRED,
    }
}

/// solosoul_log —— 写日志（SDK 签名：无返回值）
fn log_impl(
    mut caller: Caller<'_, SoloHostState>,
    level_ptr: i32,
    level_len: i32,
    message_ptr: i32,
    message_len: i32,
) {
    let level = read_string(&mut caller, level_ptr, level_len).unwrap_or_default();
    let message = read_string(&mut caller, message_ptr, message_len).unwrap_or_default();
    if level.is_empty() || message.is_empty() {
        return;
    }
    let log = PluginLogLine {
        id: uuid::Uuid::new_v4().to_string(),
        level: level.clone(),
        message: message.clone(),
        timestamp: now_millis(),
    };
    let (plugin_id, session_id) = {
        let host = &caller.data().host;
        if let Ok(mut guard) = host.logs.lock() {
            guard.push(log);
        }
        let _ = host.channel.send(PluginEvent::log(&level, &message));
        (host.plugin_id.clone(), host.session_id.clone())
    };
    caller.data().host.audit.log(
        &plugin_id,
        Some(&session_id),
        PluginAuditAction::PluginRunStarted,
    );
}

/// 工具簇：get_timestamp / get_locale / sleep / result / post_data（P223-① 分簇）
fn register_util_fns(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    // solosoul_get_timestamp —— 获取当前 Unix 时间戳（毫秒）
    linker
        .func_wrap(
            "env",
            "solosoul_get_timestamp",
            |_caller: Caller<'_, SoloHostState>| -> i64 {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_get_locale —— 获取当前 locale
    linker
        .func_wrap(
            "env",
            "solosoul_get_locale",
            |mut caller: Caller<'_, SoloHostState>,
             out_ptr: i32,
             out_len: i32,
             written_ptr: i32|
             -> i32 {
                let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
                write_buffer(&mut caller, out_ptr, out_len, &locale, written_ptr)
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_sleep —— 同步睡眠（毫秒）
    linker
        .func_wrap(
            "env",
            "solosoul_sleep",
            |_caller: Caller<'_, SoloHostState>, ms: i64| -> i32 {
                let dur = u64::try_from(ms).unwrap_or(0).min(MAX_PLUGIN_SLEEP_MS);
                std::thread::sleep(Duration::from_millis(dur));
                code::SUCCESS
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_result —— SDK 原始结果通道
    linker
        .func_wrap(
            "env",
            "solosoul_result",
            |mut caller: Caller<'_, SoloHostState>, data_ptr: i32, data_len: i32| -> i32 {
                let json = read_string(&mut caller, data_ptr, data_len).unwrap_or_default();
                let value = serde_json::from_str(&json).unwrap_or(serde_json::Value::Null);
                let host = &caller.data().host;
                // P004：盖章——`watermark_result` 载荷中的 `outputDir` 属插件可控数据，
                // 恶意插件可上报 `outputDir: "/"` 使 `resolve_output_file` 的 starts_with
                // 包含校验恒真（canonical(path).starts_with("/") 对任意路径成立），从而经
                // `plugin_open_output_file`/`plugin_copy_output_file` 打开/复制任意本地文件。
                // 此处用宿主已知的 run 参数 `outputDir`（用户配置的输出目录）覆写该字段，
                // 使后续校验的信任基准不再受插件控制。
                // P004：盖章逻辑已抽为纯函数 `stamp_result_payload`（可单测，防回归）。
                let stamped = stamp_result_payload(value, &host.params);
                let stamped_json = serde_json::to_string(&stamped).unwrap_or(json);
                {
                    let mut guard = host.results.lock().unwrap_or_else(|e| e.into_inner());
                    guard.push(PluginResultPayload(stamped));
                }
                let _ = host.channel.send(PluginEvent::result(stamped_json));
                code::SUCCESS
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_post_data —— 代理 HTTP POST 请求
    linker
        .func_wrap(
            "env",
            "solosoul_post_data",
            |mut caller: Caller<'_, SoloHostState>,
             url_ptr: i32,
             url_len: i32,
             body_ptr: i32,
             body_len: i32,
             out_ptr: i32,
             out_len: i32|
             -> i32 {
                let url = match read_required_string(&mut caller, url_ptr, url_len) {
                    Some(s) => s,
                    None => return code::INVALID_ARGUMENT,
                };
                let body = match read_string(&mut caller, body_ptr, body_len) {
                    Ok(s) => s,
                    Err(_) => return code::INVALID_ARGUMENT,
                };

                let host = &caller.data().host;
                if !host.check_rate("post_data") {
                    return code::RATE_LIMITED;
                }

                // 检查网络策略
                let policy = &host.manifest.network_policy;
                if policy.block_all_outbound {
                    return code::DOMAIN_NOT_ALLOWED;
                }

                let parsed_url = match Url::parse(&url) {
                    Ok(u) => u,
                    Err(_) => return code::INVALID_ARGUMENT,
                };
                let domain = parsed_url.host_str().unwrap_or("").to_lowercase();
                if domain.is_empty() || !is_domain_allowed(&domain, &policy.allowed_domains) {
                    return code::DOMAIN_NOT_ALLOWED;
                }

                let (plugin_id, session_id) = (host.plugin_id.clone(), host.session_id.clone());
                let client = host.http_client.clone();
                host.audit.log(
                    &plugin_id,
                    Some(&session_id),
                    PluginAuditAction::PluginRunStarted,
                );

                let response_text = match perform_http_post(&client, &url, &body) {
                    Ok(text) => text,
                    Err(e) => {
                        let _ = host.channel.send(PluginEvent::log(
                            "error",
                            format!("solosoul_post_data 失败: {}", e),
                        ));
                        return code::NETWORK_TIMEOUT;
                    }
                };

                // 截断到 64KB，避免结果过大
                let truncated: String = response_text.chars().take(64 * 1024).collect();
                write_buffer(&mut caller, out_ptr, out_len, &truncated, -1)
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    Ok(())
}
fn is_under_workspace(host: &SoloHostFunctions, path: &Path) -> bool {
    match host.workspace_dir.as_ref() {
        Some(ws) => solosoul_core::path_util::is_path_under_workspace(ws, path),
        None => false,
    }
}

fn copy_attachment_to_workspace(
    resolver: &FieldResolver,
    workspace: &Path,
    object_id: &str,
    attachment_id: &str,
) -> Result<PathBuf, String> {
    let vault = resolver
        .vault_ref()
        .ok_or_else(|| "Vault 未解锁".to_string())?;
    let _account_id = resolver
        .account_id_ref()
        .ok_or_else(|| "未选择账户".to_string())?;

    let record = vault
        .load_object(object_id)
        .map_err(|e| format!("读取对象失败: {}", e))?
        .ok_or_else(|| "对象不存在".to_string())?;

    let attachment = record
        .properties
        .get("__attachments")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find(|a| {
                a.get("id")
                    .and_then(|id| id.as_str())
                    .map(|id| id == attachment_id)
                    .unwrap_or(false)
            })
        })
        .ok_or_else(|| "附件不存在".to_string())?;

    let file_name = attachment
        .get("fileName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "附件缺少文件名".to_string())?;
    // P003 兜底净化：只保留末段路径组件，防止历史数据中存储了
    // `../../evil.txt` 型 file_name（旧版导入未净化元数据）造成遍历写。
    let safe_name = sanitize_attachment_file_name(file_name)?;

    // 优先使用附件元数据中持久化的 vault_path（由 attachment_copy_to_vault 写入）。
    // 如果没有，则回退到 data_dir/attachments/{object_id}/{attachment_id}/{file_name}，
    // 其中 data_dir 是 Vault 账户目录的父目录（VaultService::base_path）。
    let src = attachment
        .get("vaultPath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| {
            vault
                .base_path()
                .parent()
                .map(|data_dir| {
                    data_dir
                        .join("attachments")
                        .join(object_id)
                        .join(attachment_id)
                        .join(&safe_name)
                })
                .filter(|p| p.is_file())
        })
        .ok_or_else(|| {
            let fallback = vault
                .base_path()
                .parent()
                .map(|data_dir| {
                    data_dir
                        .join("attachments")
                        .join(object_id)
                        .join(attachment_id)
                        .join(&safe_name)
                })
                .unwrap_or_default();
            format!("找不到附件文件: vault_path 或 {}", fallback.display())
        })?;

    let dst_dir = workspace.join(object_id).join(attachment_id);
    let dst = dst_dir.join(&safe_name);

    std::fs::create_dir_all(&dst_dir).map_err(|e| format!("创建工作区目录失败: {}", e))?;
    std::fs::copy(&src, &dst).map_err(|e| format!("复制附件失败 ({}): {}", src.display(), e))?;

    Ok(dst)
}

/// 净化附件文件名（P003 / P023 收敛到共享实现）。
///
/// P023：语义与 `path_util::sanitize_file_name` 完全一致（平台无关拒绝 `/` `\\`
/// 分隔符 + 取末段兜底 + 拒绝空/`.`/`..`），此处直接转发，消除同款控制重复实现。
fn sanitize_attachment_file_name(file_name: &str) -> Result<String, String> {
    solosoul_core::path_util::sanitize_file_name(file_name)
}

/// P004：对插件 `solosoul_result` 载荷做「盖章」处理。
///
/// `watermark_result` 载荷中的 `outputDir` 属插件可控数据，恶意插件可上报
/// `outputDir: "/"` 使前端 `resolve_output_file` 的 `starts_with("/")` 包含校验
/// 恒真，从而经 `plugin_open_output_file`/`plugin_copy_output_file` 打开/复制任意
/// 本地文件。此处用宿主已知的 run 参数 `outputDir`（用户配置的输出目录）覆写该
/// 字段，使后续校验的信任基准不再受插件控制。纯函数便于单元测试防回归。
fn stamp_result_payload(
    value: serde_json::Value,
    params: &HashMap<String, String>,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(mut map)
            if map.get("type").and_then(|t| t.as_str()) == Some("watermark_result") =>
        {
            match params.get("outputDir").filter(|s| !s.is_empty()) {
                Some(real_dir) => {
                    map.insert(
                        "outputDir".to_string(),
                        serde_json::Value::String(real_dir.clone()),
                    );
                }
                // 宿主无真实输出目录（前端未配置/为空串）：写空串使后续
                // resolve_output_file 对空串 canonicalize 失败 → 安全拒绝，
                // 绝不透传插件自报的 outputDir。
                None => {
                    map.insert(
                        "outputDir".to_string(),
                        serde_json::Value::String(String::new()),
                    );
                }
            }
            serde_json::Value::Object(map)
        }
        other => other,
    }
}

fn write_output_file(output_dir: &Path, file_name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".."
    {
        return Err("非法文件名".to_string());
    }
    std::fs::create_dir_all(output_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let path = output_dir.join(file_name);
    std::fs::write(&path, bytes).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(path)
}

fn copy_output_file(src: &Path, output_dir: &Path, file_name: &str) -> Result<PathBuf, String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".."
    {
        return Err("非法文件名".to_string());
    }
    std::fs::create_dir_all(output_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let dst = output_dir.join(file_name);
    std::fs::copy(src, &dst).map_err(|e| format!("复制输出文件失败: {}", e))?;
    Ok(dst)
}

fn read_bytes(caller: &mut Caller<'_, SoloHostState>, ptr: i32, len: usize) -> Result<Vec<u8>, ()> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or(())?;
    let data = memory.data(&caller);
    if ptr < 0 || ptr as usize + len > data.len() {
        return Err(());
    }
    let mut buf = vec![0u8; len];
    buf.copy_from_slice(&data[ptr as usize..ptr as usize + len]);
    Ok(buf)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 从 caller 中获取 memory 导出
fn get_memory(caller: &mut Caller<'_, SoloHostState>) -> Result<Memory, PluginError> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Ok(mem),
        _ => Err(PluginError::ExecutionFailed(
            "未找到 memory 导出".to_string(),
        )),
    }
}

/// 从 Wasm 内存读取 UTF-8 字符串
fn read_string(
    caller: &mut Caller<'_, SoloHostState>,
    ptr: i32,
    len: i32,
) -> Result<String, PluginError> {
    if ptr < 0 || len < 0 {
        return Err(PluginError::InvalidArgument("非法指针".to_string()));
    }
    let len = len as usize;
    if len > MAX_PLUGIN_READ_LEN {
        return Err(PluginError::InvalidArgument(format!(
            "字符串长度超过 {} 字节限制",
            MAX_PLUGIN_READ_LEN
        )));
    }
    let mem = get_memory(caller)?;
    let mut buf = vec![0u8; len];
    mem.read(&mut *caller, ptr as usize, &mut buf)
        .map_err(|e| PluginError::ExecutionFailed(format!("读取内存失败: {}", e)))?;
    String::from_utf8(buf).map_err(|_| PluginError::InvalidManifest("非法 UTF-8".to_string()))
}

/// 读取必填字符串参数；空串或读取失败返回 `None`，调用方应返回 `code::INVALID_ARGUMENT`
fn read_required_string(
    caller: &mut Caller<'_, SoloHostState>,
    ptr: i32,
    len: i32,
) -> Option<String> {
    match read_string(caller, ptr, len) {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

/// 将 UTF-8 字符串写入 Wasm 内存，并以 `\0` 结尾
///
/// `written_ptr` 为 -1 时不回写已写入长度
fn write_buffer(
    caller: &mut Caller<'_, SoloHostState>,
    ptr: i32,
    cap: i32,
    value: &str,
    written_ptr: i32,
) -> i32 {
    if ptr < 0 || cap <= 0 {
        return code::INVALID_ARGUMENT;
    }
    // 需要为结尾的 \0 预留一字节
    if value.len() + 1 > cap as usize {
        return code::BUFFER_TOO_SMALL;
    }
    let mem = match get_memory(caller) {
        Ok(m) => m,
        Err(_) => return code::WASM_TRAP,
    };
    if mem
        .write(&mut *caller, ptr as usize, value.as_bytes())
        .is_err()
    {
        return code::WASM_TRAP;
    }
    if mem
        .write(&mut *caller, ptr as usize + value.len(), &[0])
        .is_err()
    {
        return code::WASM_TRAP;
    }
    if written_ptr >= 0 {
        let len_bytes = (value.len() as u32).to_le_bytes();
        let _ = mem.write(&mut *caller, written_ptr as usize, &len_bytes);
    }
    code::SUCCESS
}

/// 将 `PluginError` 映射为 SDK 错误码
fn plugin_error_code(err: &PluginError) -> i32 {
    match err {
        PluginError::ExecutionFailed(msg) if msg.contains("Vault 未解锁") => code::VAULT_LOCKED,
        PluginError::ExecutionFailed(msg) if msg.contains("未选择账户") => code::VAULT_LOCKED,
        PluginError::InvalidField(_) => code::INVALID_FIELD,
        PluginError::InvalidArgument(_) => code::INVALID_ARGUMENT,
        PluginError::RateLimited => code::RATE_LIMITED,
        PluginError::ConsentDenied => code::USER_DENIED,
        _ => code::INVALID_ARGUMENT,
    }
}

/// 将 u32 handle 值以 little-endian 写入 Wasm 内存
fn write_u32(caller: &mut Caller<'_, SoloHostState>, ptr: i32, value: u32) -> i32 {
    if ptr < 0 {
        return code::INVALID_ARGUMENT;
    }
    let mem = match get_memory(caller) {
        Ok(m) => m,
        Err(_) => return code::WASM_TRAP,
    };
    if mem
        .write(&mut *caller, ptr as usize, &value.to_le_bytes())
        .is_err()
    {
        return code::WASM_TRAP;
    }
    code::SUCCESS
}

/// 检查域名是否在白名单中
fn is_domain_allowed(domain: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }
    allowed
        .iter()
        .any(|pattern| super::manifest::matches_domain(domain, pattern))
}

/// 将异步 HTTP 轮询结果写入 Wasm 内存
fn write_http_poll_result(
    caller: &mut Caller<'_, SoloHostState>,
    status_ptr: i32,
    len_ptr: i32,
    result: &HttpResult,
) -> i32 {
    // inline u16 write
    if status_ptr >= 0 {
        if let Ok(mem) = get_memory(caller) {
            let _ = mem.write(
                &mut *caller,
                status_ptr as usize,
                &result.status.to_le_bytes(),
            );
        }
    }
    // inline u32 write
    if len_ptr >= 0 {
        if let Ok(mem) = get_memory(caller) {
            let _ = mem.write(
                &mut *caller,
                len_ptr as usize,
                &(result.body.len() as u32).to_le_bytes(),
            );
        }
    }
    result.error_code.unwrap_or(code::SUCCESS)
}

/// 执行异步 HTTP 请求
async fn perform_http_async(
    client: &reqwest::Client,
    method: &str,
    url: &str,
    body: &str,
) -> Result<HttpResult, i32> {
    let mut headers = HeaderMap::new();
    if serde_json::from_str::<serde_json::Value>(body).is_ok() {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    }

    let method =
        reqwest::Method::from_bytes(method.as_bytes()).map_err(|_| code::INVALID_ARGUMENT)?;

    let mut req = client.request(method, url).headers(headers);
    if !body.is_empty() {
        req = req.body(body.to_string());
    }

    let resp = req
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|_| code::NETWORK_TIMEOUT)?;

    let status = resp.status().as_u16();
    let body = resp.text().await.map_err(|_| code::NETWORK_TIMEOUT)?;

    Ok(HttpResult {
        status,
        body,
        error_code: None,
    })
}

/// 执行同步阻塞的 HTTP POST 请求
fn perform_http_post(client: &reqwest::Client, url: &str, body: &str) -> Result<String, String> {
    let mut headers = HeaderMap::new();
    if serde_json::from_str::<serde_json::Value>(body).is_ok() {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    }

    tokio::runtime::Handle::try_current()
        .map_err(|_| "无法在当前线程执行网络请求".to_string())?
        .block_on(async {
            let resp = client
                .post(url)
                .headers(headers)
                .body(body.to_string())
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let text = resp.text().await.map_err(|e| e.to_string())?;
            Ok(text)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn test_sanitize_attachment_file_name_accepts_normal_name() {
        let result = sanitize_attachment_file_name("doc.pdf");
        assert_eq!(result.unwrap(), "doc.pdf");
        // "..." 是合法文件名（非空、非 . 非 ..），应被接受
        assert_eq!(sanitize_attachment_file_name("...").unwrap(), "...");
    }

    #[test]
    fn test_sanitize_attachment_file_name_rejects_path_separators() {
        // P003 平台无关拒绝：正斜杠与反斜杠（Windows 反斜杠分隔符）
        assert!(sanitize_attachment_file_name("../../evil.txt").is_err());
        assert!(sanitize_attachment_file_name("..\\..\\evil.txt").is_err());
        assert!(sanitize_attachment_file_name("a/b.txt").is_err());
        assert!(sanitize_attachment_file_name("a\\b.txt").is_err());
    }

    #[test]
    fn test_sanitize_attachment_file_name_rejects_dot_and_empty() {
        assert!(sanitize_attachment_file_name("").is_err());
        assert!(sanitize_attachment_file_name(".").is_err());
        assert!(sanitize_attachment_file_name("..").is_err());
    }

    #[test]
    fn test_stamp_result_payload_watermark_overrides_with_host_dir() {
        let payload = serde_json::json!({
            "type": "watermark_result",
            "outputDir": "/",
            "file": "doc.pdf"
        });
        let params = HashMap::from([("outputDir".to_string(), "/Users/me/Desktop".to_string())]);
        let stamped = stamp_result_payload(payload, &params);
        assert_eq!(stamped["outputDir"], "/Users/me/Desktop");
        assert_eq!(stamped["type"], "watermark_result");
        assert_eq!(stamped["file"], "doc.pdf");
    }

    #[test]
    fn test_stamp_result_payload_watermark_empty_host_dir_writes_empty() {
        let payload = serde_json::json!({
            "type": "watermark_result",
            "outputDir": "/etc"
        });
        // 宿主未配置 outputDir：写空串使后续 resolve_output_file 失败 → 安全拒绝
        let params = HashMap::new();
        let stamped = stamp_result_payload(payload, &params);
        assert_eq!(stamped["outputDir"], "");
    }

    #[test]
    fn test_stamp_result_payload_watermark_empty_string_host_dir_writes_empty() {
        let payload = serde_json::json!({
            "type": "watermark_result",
            "outputDir": "/etc"
        });
        // params 含 outputDir 但为空串：`.filter(|s| !s.is_empty())` 边界 → 同 None 分支写空串
        let params = HashMap::from([("outputDir".to_string(), String::new())]);
        let stamped = stamp_result_payload(payload, &params);
        assert_eq!(stamped["outputDir"], "");
    }

    #[test]
    fn test_stamp_result_payload_non_watermark_passthrough() {
        let payload = serde_json::json!({
            "type": "chat_result",
            "outputDir": "/whatever"
        });
        let params = HashMap::new();
        let stamped = stamp_result_payload(payload, &params);
        // 非 watermark_result 载荷原样透传，不盖章
        assert_eq!(stamped["outputDir"], "/whatever");
        assert_eq!(stamped["type"], "chat_result");
    }

    #[tokio::test]
    async fn test_perform_http_async_get() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _n = socket.read(&mut buf).await.unwrap();
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
            socket.write_all(response).await.unwrap();
        });

        let url = format!("http://{}", addr);
        let client = reqwest::Client::new();
        let result = perform_http_async(&client, "GET", &url, "").await.unwrap();
        assert_eq!(result.status, 200);
        assert_eq!(result.body, "hello");
    }

    #[tokio::test]
    async fn test_perform_http_async_post_json() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 2048];
            let _n = socket.read(&mut buf).await.unwrap();
            let response = b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\nOK";
            socket.write_all(response).await.unwrap();
        });

        let url = format!("http://{}", addr);
        let client = reqwest::Client::new();
        let result = perform_http_async(&client, "POST", &url, r#"{"name":"Alice"}"#)
            .await
            .unwrap();
        assert_eq!(result.status, 201);
        assert_eq!(result.body, "OK");
    }

    #[tokio::test]
    async fn test_p003_redirect_not_followed() {
        // P003 回归：生产 client 关闭自动跟随重定向——白名单只校验初始 URL，
        // 跟随 302 即可把请求引到白名单外主机。断言 3xx 原样返回且不请求 Location。
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let redirect_url = format!("http://{}/target", addr);

        let hits = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let hits_clone = hits.clone();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 2048];
            let _n = socket.read(&mut buf).await.unwrap();
            hits_clone.store(1, std::sync::atomic::Ordering::SeqCst);
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: {redirect_url}\r\nContent-Length: 0\r\n\r\n"
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        // 与生产路径同款策略（Policy::none），非默认跟随
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let url = format!("http://{}", addr);
        let result = perform_http_async(&client, "GET", &url, "").await.unwrap();
        assert_eq!(result.status, 302, "302 应原样返回而非跟随");
        // 未跟随 → 服务器只收到一次请求（不会有对 /target 的第二次请求）
        assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
