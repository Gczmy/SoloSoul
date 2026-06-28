use super::{code, HttpResult, PluginError, SoloHostState, MAX_PLUGIN_READ_LEN};
use wasmtime::{Caller, Extern, Memory};

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 从 caller 中获取 memory 导出
pub(crate) fn get_memory(caller: &mut Caller<'_, SoloHostState>) -> Result<Memory, PluginError> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Ok(mem),
        _ => Err(PluginError::ExecutionFailed(
            "未找到 memory 导出".to_string(),
        )),
    }
}

/// 从 Wasm 内存读取 UTF-8 字符串
pub(crate) fn read_string(
    caller: &mut Caller<'_, SoloHostState>,
    ptr: i32,
    len: i32,
) -> Result<String, PluginError> {
    let bytes = read_bytes(caller, ptr, len)?;
    String::from_utf8(bytes).map_err(|_| PluginError::InvalidManifest("非法 UTF-8".to_string()))
}

/// 从 Wasm 内存读取原始字节
pub(crate) fn read_bytes(
    caller: &mut Caller<'_, SoloHostState>,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, PluginError> {
    if ptr < 0 || len < 0 {
        return Err(PluginError::InvalidArgument("非法指针".to_string()));
    }
    let len = len as usize;
    if len > MAX_PLUGIN_READ_LEN {
        return Err(PluginError::InvalidArgument(format!(
            "读取长度超过 {} 字节限制",
            MAX_PLUGIN_READ_LEN
        )));
    }
    let mem = get_memory(caller)?;
    let mut buf = vec![0u8; len];
    mem.read(&mut *caller, ptr as usize, &mut buf)
        .map_err(|e| PluginError::ExecutionFailed(format!("读取内存失败: {}", e)))?;
    Ok(buf)
}

/// 将 UTF-8 字符串写入 Wasm 内存，并以 `\0` 结尾
///
/// `written_ptr` 为 -1 时不回写已写入长度
pub(crate) fn write_buffer(
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
pub(crate) fn plugin_error_code(err: &PluginError) -> i32 {
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

/// 检查域名是否在白名单中
pub(crate) fn is_domain_allowed(domain: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return false;
    }
    allowed
        .iter()
        .any(|pattern| crate::plugin::manifest::matches_domain(domain, pattern))
}

/// 将 u16 以 little-endian 写入 Wasm 内存
pub(crate) fn write_u16(caller: &mut Caller<'_, SoloHostState>, ptr: i32, value: u16) -> i32 {
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

/// 将 u32 以 little-endian 写入 Wasm 内存
pub(crate) fn write_u32(caller: &mut Caller<'_, SoloHostState>, ptr: i32, value: u32) -> i32 {
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

/// 将异步 HTTP 轮询结果写入 Wasm 内存
pub(crate) fn write_http_poll_result(
    caller: &mut Caller<'_, SoloHostState>,
    status_ptr: i32,
    len_ptr: i32,
    result: &HttpResult,
) -> i32 {
    let _ = write_u16(caller, status_ptr, result.status);
    let _ = write_u32(caller, len_ptr, result.body.len() as u32);
    result.error_code.unwrap_or(code::SUCCESS)
}
