use super::http::{block_on, perform_http_async, perform_http_post};
use super::memory::{
    is_domain_allowed, now_millis, plugin_error_code, read_bytes, read_string, write_buffer,
    write_http_poll_result, write_u32,
};
use super::{
    code, oneshot, Duration, HttpHandleState, HttpResult, Ordering, PluginAuditAction, PluginError,
    PluginEvent, PluginLogLine, PluginResultPayload, SoloHostState, Url, MAX_PLUGIN_SLEEP_MS,
};
use std::path::PathBuf;
use wasmtime::{Caller, Linker};

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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_list_objects —— 列出指定类型的所有对象（Phase 5，替代 .count）
    linker
        .func_wrap(
            "env",
            "solosoul_list_objects",
            |mut caller: Caller<'_, SoloHostState>,
             type_id_ptr: i32,
             type_id_len: i32,
             out_ptr: i32,
             out_cap: i32|
             -> i32 {
                let type_id = match read_string(&mut caller, type_id_ptr, type_id_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
                };
                let (plugin_id, session_id) = {
                    let host = &caller.data().host;
                    if !host.rate_limiter.check(&host.plugin_id, "list_objects") {
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
                let url = match read_string(&mut caller, url_ptr, url_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
                };
                let body = match read_string(&mut caller, body_ptr, body_len) {
                    Ok(s) => s,
                    Err(_) => return code::INVALID_ARGUMENT,
                };

                let host = &caller.data().host;
                if !host.rate_limiter.check(&host.plugin_id, "post_data") {
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

    // solosoul_http_request —— 发起异步 HTTP 请求（返回句柄）
    linker
        .func_wrap(
            "env",
            "solosoul_http_request",
            |mut caller: Caller<'_, SoloHostState>,
             method_ptr: i32,
             method_len: i32,
             url_ptr: i32,
             url_len: i32,
             body_ptr: i32,
             body_len: i32,
             out_handle_ptr: i32|
             -> i32 {
                let method = match read_string(&mut caller, method_ptr, method_len) {
                    Ok(s) if !s.is_empty() => s.to_uppercase(),
                    _ => return code::INVALID_ARGUMENT,
                };
                let url = match read_string(&mut caller, url_ptr, url_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
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
                    if !host.rate_limiter.check(&host.plugin_id, "http_request") {
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
                        || !is_domain_allowed(
                            &domain,
                            &host.manifest.network_policy.allowed_domains,
                        )
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
                        let result =
                            perform_http_async(&client, &method_clone, &url_clone, &body).await;
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
                        let mut handles =
                            host.http_handles.lock().unwrap_or_else(|e| e.into_inner());
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_http_poll —— 轮询异步 HTTP 请求状态
    linker
        .func_wrap(
            "env",
            "solosoul_http_poll",
            |mut caller: Caller<'_, SoloHostState>,
             handle: i32,
             out_status_ptr: i32,
             out_len_ptr: i32|
             -> i32 {
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_http_read —— 读取异步 HTTP 响应体
    linker
        .func_wrap(
            "env",
            "solosoul_http_read",
            |mut caller: Caller<'_, SoloHostState>,
             handle: i32,
             out_ptr: i32,
             out_cap: i32,
             written_ptr: i32|
             -> i32 {
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_http_close —— 关闭并释放异步 HTTP 句柄
    linker
        .func_wrap(
            "env",
            "solosoul_http_close",
            |caller: Caller<'_, SoloHostState>, handle: i32| -> i32 {
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
            },
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

    // solosoul_get_data_structure_tree —— 数据结构树（元数据）
    linker
        .func_wrap(
            "env",
            "solosoul_get_data_structure_tree",
            |mut caller: Caller<'_, SoloHostState>, out_ptr: i32, out_len: i32| -> i32 {
                let (plugin_id, session_id) = {
                    let host = &caller.data().host;
                    if !host
                        .rate_limiter
                        .check(&host.plugin_id, "get_data_structure_tree")
                    {
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
                let stamped = match value {
                    serde_json::Value::Object(mut map)
                        if map.get("type").and_then(|t| t.as_str()) == Some("watermark_result") =>
                    {
                        match host.params.get("outputDir").filter(|s| !s.is_empty()) {
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
                };
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

    // solosoul_show_dialog —— 通用对话框（阻塞等待用户响应）
    linker
        .func_wrap(
            "env",
            "solosoul_show_dialog",
            |mut caller: Caller<'_, SoloHostState>,
             config_ptr: i32,
             config_len: i32,
             out_ptr: i32,
             out_len: i32|
             -> i32 {
                let config = match read_string(&mut caller, config_ptr, config_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
                };
                if config.len() > 4096 {
                    return code::INVALID_ARGUMENT;
                }

                let request_id = uuid::Uuid::new_v4().to_string();
                let (plugin_id, plugin_name, session_id, consent_manager) = {
                    let host = &caller.data().host;
                    if !host.rate_limiter.check(&host.plugin_id, "show_dialog") {
                        return code::RATE_LIMITED;
                    }
                    (
                        host.plugin_id.clone(),
                        host.plugin_name.clone(),
                        host.session_id.clone(),
                        host.consent_manager.clone(),
                    )
                };

                let event =
                    PluginEvent::dialog_request(&request_id, &plugin_id, &plugin_name, &config);
                let _ = caller.data().host.channel.send(event);
                caller.data().host.audit.log(
                    &plugin_id,
                    Some(&session_id),
                    PluginAuditAction::PluginRunStarted,
                );

                let rx = match block_on(consent_manager.request_consent(&request_id)) {
                    Ok(rx) => rx,
                    Err(_) => return code::NOT_IMPLEMENTED,
                };

                match block_on(tokio::time::timeout(Duration::from_secs(300), rx)) {
                    Ok(Ok(Ok(Some(value)))) => {
                        write_buffer(&mut caller, out_ptr, out_len, &value, -1)
                    }
                    Ok(Ok(Ok(None))) => code::USER_DENIED,
                    Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => code::TTL_EXPIRED,
                }
            },
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
                let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
                write_buffer(&mut caller, out_ptr, out_len, &locale, written_ptr)
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_request_consent —— 请求用户授权（阻塞等待用户响应）
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
                if field_id.is_empty() || request_id.is_empty() {
                    return code::INVALID_ARGUMENT;
                }

                let (plugin_id, plugin_name, session_id, consent_manager) = {
                    let host = &caller.data().host;
                    if !host.rate_limiter.check(&host.plugin_id, "request_consent") {
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
                let rx = match block_on(consent_manager.request_consent(&request_id)) {
                    Ok(rx) => rx,
                    Err(_) => return code::NOT_IMPLEMENTED,
                };

                match block_on(tokio::time::timeout(Duration::from_secs(300), rx)) {
                    Ok(Ok(Ok(Some(_value)))) => {
                        caller.data().host.audit.log(
                            &plugin_id,
                            Some(&session_id),
                            PluginAuditAction::ConsentApproved { field_id },
                        );
                        code::SUCCESS
                    }
                    Ok(Ok(Ok(None))) => {
                        caller.data().host.audit.log(
                            &plugin_id,
                            Some(&session_id),
                            PluginAuditAction::ConsentDenied { field_id },
                        );
                        code::USER_DENIED
                    }
                    Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
                        caller.data().host.audit.log(
                            &plugin_id,
                            Some(&session_id),
                            PluginAuditAction::ConsentDenied { field_id },
                        );
                        code::TTL_EXPIRED
                    }
                }
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

    // solosoul_list_attachments —— 列出可水印的附件树
    linker
        .func_wrap(
            "env",
            "solosoul_list_attachments",
            |mut caller: Caller<'_, SoloHostState>, out_ptr: i32, out_cap: i32| -> i32 {
                let resolver = caller.data().host.field_resolver.clone();
                match resolver.list_attachments() {
                    Ok(json) => write_buffer(&mut caller, out_ptr, out_cap, &json, -1),
                    Err(e) => plugin_error_code(&e),
                }
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_prepare_attachment_copy —— 将 Vault 附件复制到插件临时工作区
    linker
        .func_wrap(
            "env",
            "solosoul_prepare_attachment_copy",
            |mut caller: Caller<'_, SoloHostState>,
             object_id_ptr: i32,
             object_id_len: i32,
             attachment_id_ptr: i32,
             attachment_id_len: i32,
             out_path_ptr: i32,
             out_path_cap: i32|
             -> i32 {
                let object_id = match read_string(&mut caller, object_id_ptr, object_id_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
                };
                let attachment_id =
                    match read_string(&mut caller, attachment_id_ptr, attachment_id_len) {
                        Ok(s) if !s.is_empty() => s,
                        _ => return code::INVALID_ARGUMENT,
                    };

                let workspace = match caller.data().host.workspace_dir.as_ref() {
                    Some(d) => d.clone(),
                    None => return code::NOT_IMPLEMENTED,
                };

                let resolver = caller.data().host.field_resolver.clone();
                match copy_attachment_to_workspace(
                    &resolver,
                    &workspace,
                    &object_id,
                    &attachment_id,
                ) {
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_image_watermark —— 为图片添加水印
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    linker
        .func_wrap(
            "env",
            "solosoul_image_watermark",
            |mut caller: Caller<'_, SoloHostState>,
             input_path_ptr: i32,
             input_path_len: i32,
             output_path_ptr: i32,
             output_path_len: i32,
             config_json_ptr: i32,
             config_json_len: i32|
             -> i32 {
                let input_path = match read_string(&mut caller, input_path_ptr, input_path_len) {
                    Ok(s) if !s.is_empty() => PathBuf::from(s),
                    _ => return code::INVALID_ARGUMENT,
                };
                let output_path = match read_string(&mut caller, output_path_ptr, output_path_len) {
                    Ok(s) if !s.is_empty() => PathBuf::from(s),
                    _ => return code::INVALID_ARGUMENT,
                };
                let config_json = match read_string(&mut caller, config_json_ptr, config_json_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
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
                                format!("image_watermark 配置解析失败: {}", e),
                            ));
                            return code::INVALID_ARGUMENT;
                        }
                    };

                match solosoul_core::watermark::apply_to_image(&input_path, &output_path, &config) {
                    Ok(()) => code::SUCCESS,
                    Err(e) => {
                        let _ = caller.data().host.channel.send(PluginEvent::log(
                            "error",
                            format!("image_watermark 失败: {}", e),
                        ));
                        code::PROCESSING_FAILED
                    }
                }
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

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
    linker
        .func_wrap(
            "env",
            "solosoul_pdf_watermark",
            |mut caller: Caller<'_, SoloHostState>,
             input_path_ptr: i32,
             input_path_len: i32,
             output_path_ptr: i32,
             output_path_len: i32,
             config_json_ptr: i32,
             config_json_len: i32|
             -> i32 {
                let input_path = match read_string(&mut caller, input_path_ptr, input_path_len) {
                    Ok(s) if !s.is_empty() => PathBuf::from(s),
                    _ => return code::INVALID_ARGUMENT,
                };
                let output_path = match read_string(&mut caller, output_path_ptr, output_path_len) {
                    Ok(s) if !s.is_empty() => PathBuf::from(s),
                    _ => return code::INVALID_ARGUMENT,
                };
                let config_json = match read_string(&mut caller, config_json_ptr, config_json_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
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
                                format!("pdf_watermark 配置解析失败: {}", e),
                            ));
                            return code::INVALID_ARGUMENT;
                        }
                    };

                match solosoul_core::watermark::apply_to_pdf(&input_path, &output_path, &config) {
                    Ok(()) => code::SUCCESS,
                    Err(e) => {
                        let _ = caller.data().host.channel.send(PluginEvent::log(
                            "error",
                            format!("pdf_watermark 失败: {}", e),
                        ));
                        code::PROCESSING_FAILED
                    }
                }
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

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

    // solosoul_write_output_file —— 将字节写入输出目录
    linker
        .func_wrap(
            "env",
            "solosoul_write_output_file",
            |mut caller: Caller<'_, SoloHostState>,
             file_name_ptr: i32,
             file_name_len: i32,
             bytes_ptr: i32,
             bytes_len: i32,
             out_path_ptr: i32,
             out_path_cap: i32|
             -> i32 {
                let file_name = match read_string(&mut caller, file_name_ptr, file_name_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
                };
                if bytes_len < 0 || bytes_len as usize > 256 * 1024 * 1024 {
                    return code::FILE_TOO_LARGE;
                }
                let bytes_len = bytes_len as usize;
                let bytes = match read_bytes(&mut caller, bytes_ptr, bytes_len as i32) {
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    // solosoul_copy_output_file —— 将工作区中的已处理文件复制到输出目录
    linker
        .func_wrap(
            "env",
            "solosoul_copy_output_file",
            |mut caller: Caller<'_, SoloHostState>,
             src_path_ptr: i32,
             src_path_len: i32,
             file_name_ptr: i32,
             file_name_len: i32,
             out_path_ptr: i32,
             out_path_cap: i32|
             -> i32 {
                let src_path = match read_string(&mut caller, src_path_ptr, src_path_len) {
                    Ok(s) if !s.is_empty() => PathBuf::from(s),
                    _ => return code::INVALID_ARGUMENT,
                };
                let file_name = match read_string(&mut caller, file_name_ptr, file_name_len) {
                    Ok(s) if !s.is_empty() => s,
                    _ => return code::INVALID_ARGUMENT,
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
            },
        )
        .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

    Ok(())
}

fn is_under_workspace(host: &super::SoloHostFunctions, path: &std::path::Path) -> bool {
    match host.workspace_dir.as_ref() {
        Some(ws) => solosoul_core::path_util::is_path_under_workspace(ws, path),
        None => false,
    }
}

fn copy_attachment_to_workspace(
    resolver: &super::super::FieldResolver,
    workspace: &std::path::Path,
    object_id: &str,
    attachment_id: &str,
) -> Result<PathBuf, String> {
    let vault = resolver
        .vault_ref()
        .ok_or_else(|| "Vault 未解锁".to_string())?;

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
        .map(std::path::PathBuf::from)
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

/// 净化附件文件名（P003）：平台无关地拒绝含分隔符的名字
/// （Unix 上 `\\` 不是分隔符，仅靠 `Path::file_name()` 无法剥离
/// `..\\..\\evil.txt` 中的反斜杠），再取末段组件作为兜底。
fn sanitize_attachment_file_name(file_name: &str) -> Result<String, String> {
    if file_name.contains('/') || file_name.contains('\\') {
        return Err(format!("附件文件名无效: {}", file_name));
    }
    let safe = std::path::Path::new(file_name)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty() && s != "." && s != "..")
        .ok_or_else(|| format!("附件文件名无效: {}", file_name))?;
    Ok(safe)
}

fn write_output_file(
    output_dir: &std::path::Path,
    file_name: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".."
    {
        return Err("非法文件名".to_string());
    }
    std::fs::create_dir_all(output_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let path = output_dir.join(file_name);
    std::fs::write(&path, bytes).map_err(|e| format!("写入文件失败: {}", e))?;
    Ok(path)
}

fn copy_output_file(
    src: &std::path::Path,
    output_dir: &std::path::Path,
    file_name: &str,
) -> Result<PathBuf, String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name == "." || file_name == ".."
    {
        return Err("非法文件名".to_string());
    }
    std::fs::create_dir_all(output_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;
    let dst = output_dir.join(file_name);
    std::fs::copy(src, &dst).map_err(|e| format!("复制输出文件失败: {}", e))?;
    Ok(dst)
}
