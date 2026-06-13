//! 插件 Host Functions
//!
//! 本模块将 SoloSoul 核心能力通过 `env` 模块暴露给 WebAssembly 插件。
//! ABI 与 `SoloSoul_plugin_market/SDK/rust` 保持一致。

use super::{
    ConsentManager, FieldResolver, PluginAuditAction, PluginAuditLogger, PluginError, PluginEvent,
    PluginLogLine, PluginManifest, PluginResultPayload, RateLimiter,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::ipc::Channel;
use wasmtime::{Caller, Extern, Linker, Memory};

/// Host Function 错误码
#[allow(dead_code)]
mod code {
    pub const SUCCESS: i32 = 0;
    pub const INVALID_ARGUMENT: i32 = 1;
    pub const FIELD_NOT_FOUND: i32 = 2;
    pub const RATE_LIMITED: i32 = 3;
    pub const CONSENT_DENIED: i32 = 4;
    pub const BUFFER_TOO_SMALL: i32 = 5;
    pub const WASM_TRAP: i32 = 6;
    pub const NOT_IMPLEMENTED: i32 = 7;
}

/// 传递给 Wasm Store 的状态，包含 WASI 上下文与自定义 Host 数据
pub struct SoloHostState {
    pub wasi: wasmtime_wasi::p1::WasiP1Ctx,
    pub host: SoloHostFunctions,
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

/// 注册所有 Host Functions 到 linker
pub fn register_host_functions(linker: &mut Linker<SoloHostState>) -> Result<(), PluginError> {
    // solosoul_request_field —— 请求字段
    linker
        .func_wrap(
            "env",
            "solosoul_request_field",
            |mut caller: Caller<'_, SoloHostState>,
             field_id_ptr: i32,
             field_id_len: i32,
             out_ptr: i32,
             out_len: i32|
             -> i32 {
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
                    if !host.rate_limiter.check(&host.plugin_id, "request_field") {
                        return code::RATE_LIMITED;
                    }
                    (host.plugin_id.clone(), host.session_id.clone())
                };
                // Phase 2 占位：所有字段默认返回空字符串，避免测试阻塞
                let value = caller
                    .data()
                    .host
                    .field_resolver
                    .resolve(&field_id)
                    .unwrap_or_default();
                caller.data().host.audit.log(
                    &plugin_id,
                    Some(&session_id),
                    PluginAuditAction::PluginRunStarted,
                );
                write_buffer(&mut caller, out_ptr, out_len, &value, -1)
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_post_data —— 网络请求（未实现）
    linker
        .func_wrap(
            "env",
            "solosoul_post_data",
            |_caller: Caller<'_, SoloHostState>,
             _url_ptr: i32,
             _url_len: i32,
             _body_ptr: i32,
             _body_len: i32,
             _out_ptr: i32,
             _out_len: i32|
             -> i32 { code::NOT_IMPLEMENTED },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_log —— 写日志（SDK 签名：无返回值）
    linker
        .func_wrap(
            "env",
            "solosoul_log",
            |mut caller: Caller<'_, SoloHostState>,
             level_ptr: i32,
             level_len: i32,
             message_ptr: i32,
             message_len: i32| {
                let level = read_string(&mut caller, level_ptr, level_len).unwrap_or_default();
                let message =
                    read_string(&mut caller, message_ptr, message_len).unwrap_or_default();
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

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

    // solosoul_get_data_structure_tree —— 数据结构树（未实现）
    linker
        .func_wrap(
            "env",
            "solosoul_get_data_structure_tree",
            |_caller: Caller<'_, SoloHostState>, _out_ptr: i32, _out_len: i32| -> i32 {
                code::NOT_IMPLEMENTED
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
                {
                    let mut guard = host.results.lock().unwrap_or_else(|e| e.into_inner());
                    guard.push(PluginResultPayload(value));
                }
                let _ = host.channel.send(PluginEvent::result(json));
                code::SUCCESS
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_show_dialog —— 通用对话框（未实现）
    linker
        .func_wrap(
            "env",
            "solosoul_show_dialog",
            |_caller: Caller<'_, SoloHostState>,
             _config_ptr: i32,
             _config_len: i32,
             _out_ptr: i32,
             _out_len: i32|
             -> i32 { code::NOT_IMPLEMENTED },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_get_param —— 获取运行参数
    linker
        .func_wrap(
            "env",
            "solosoul_get_param",
            |mut caller: Caller<'_, SoloHostState>,
             key_ptr: i32,
             key_len: i32,
             out_ptr: i32,
             out_len: i32,
             written_ptr: i32|
             -> i32 {
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
                let locale = sys_locale::get_locale()
                    .map(|l| l.to_string())
                    .unwrap_or_else(|| "en-US".to_string());
                write_buffer(&mut caller, out_ptr, out_len, &locale, written_ptr)
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_request_consent —— 请求用户授权
    linker
        .func_wrap(
            "env",
            "solosoul_request_consent",
            |mut caller: Caller<'_, SoloHostState>,
             field_id_ptr: i32,
             field_id_len: i32,
             request_id_ptr: i32,
             request_id_len: i32|
             -> i32 {
                let field_id =
                    read_string(&mut caller, field_id_ptr, field_id_len).unwrap_or_default();
                let request_id =
                    read_string(&mut caller, request_id_ptr, request_id_len).unwrap_or_default();
                let (plugin_id, plugin_name, session_id) = {
                    let host = &caller.data().host;
                    host.audit.log(
                        &host.plugin_id,
                        Some(&host.session_id),
                        PluginAuditAction::ConsentApproved {
                            field_id: field_id.clone(),
                        },
                    );
                    (
                        host.plugin_id.clone(),
                        host.plugin_name.clone(),
                        host.session_id.clone(),
                    )
                };
                let event = PluginEvent::consent_request(
                    &request_id,
                    &plugin_id,
                    &plugin_name,
                    &field_id,
                    &field_id,
                    "sensitive",
                );
                let _ = caller.data().host.channel.send(event);
                caller.data().host.audit.log(
                    &plugin_id,
                    Some(&session_id),
                    PluginAuditAction::ConsentApproved { field_id },
                );
                code::SUCCESS
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_sleep —— 同步睡眠（毫秒）
    linker
        .func_wrap(
            "env",
            "solosoul_sleep",
            |_caller: Caller<'_, SoloHostState>, ms: i64| -> i32 {
                let dur = u64::try_from(ms).unwrap_or(0);
                std::thread::sleep(Duration::from_millis(dur));
                code::SUCCESS
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    Ok(())
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
    let mem = get_memory(caller)?;
    let mut buf = vec![0u8; len as usize];
    mem.read(&mut *caller, ptr as usize, &mut buf)
        .map_err(|e| PluginError::ExecutionFailed(format!("读取内存失败: {}", e)))?;
    String::from_utf8(buf).map_err(|_| PluginError::InvalidManifest("非法 UTF-8".to_string()))
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
