//! Host functions - Secure interface between Wasm and Rust Core
//!
//! Plugins can only access data through these strictly defined functions.
//! All access is logged, rate-limited, and subject to user consent.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use wasmtime::{Caller, Linker};

use super::manifest::PluginManifest;
use super::rust_log;
use crate::vault::{ProfileData, VersionedProfileData};

// ============================================================================
// Consent Channel
// ============================================================================

/// 请求 Flutter 层用户确认
#[derive(Debug, Clone)]
pub struct ConsentChannel {
    pub tx: mpsc::Sender<ConsentRequest>,
}

/// 单次字段访问的授权请求
#[derive(Debug)]
pub struct ConsentRequest {
    pub request_id: String,
    pub plugin_id: String,
    pub plugin_name: String,
    pub field: String,
    pub session_id: String,
    pub sensitivity: SensitivityLevel,
    pub response: std::sync::mpsc::Sender<ConsentResult>,
}

/// 用户授权结果
#[derive(Debug, Clone)]
pub enum ConsentResult {
    /// 用户授权，返回解密后的字段值
    Approved(String),
    /// 用户拒绝
    Denied,
    /// 超时（Rust 侧 60s 未收到响应）
    Expired,
}

// ============================================================================
// Sensitivity Level
// ============================================================================

/// 字段敏感度分级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SensitivityLevel {
    /// 公开数据，无需确认
    Public = 0,
    /// 内部数据，无需确认
    Internal = 1,
    /// 敏感数据，需要用户确认
    Sensitive = 2,
    /// 关键数据，需要用户确认
    Critical = 3,
}

impl SensitivityLevel {
    /// 是否需要用户显式确认
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, SensitivityLevel::Sensitive | SensitivityLevel::Critical)
    }

    /// 数值比较：self > other
    pub fn is_stricter_than(&self, other: SensitivityLevel) -> bool {
        (*self as u8) > (other as u8)
    }

    /// 从字符串解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(SensitivityLevel::Public),
            "internal" => Some(SensitivityLevel::Internal),
            "sensitive" => Some(SensitivityLevel::Sensitive),
            "critical" => Some(SensitivityLevel::Critical),
            _ => None,
        }
    }
}

/// 根据字段路径解析敏感度（运行时映射表）
pub(crate) fn resolve_field_sensitivity(field_id: &str) -> SensitivityLevel {
    match field_id {
        "identity.full_name" => SensitivityLevel::Public,
        "identity.contact.emails" | "identity.contact.phones" => SensitivityLevel::Internal,
        "identity.id_card.number"
        | "travel.primary_passport.number"
        | "financial.primary_bank_account.number" => SensitivityLevel::Critical,
        // address: country/count/title/label 为公开，其余为敏感
        "address.country" | "address.count" | "address.title" | "address.label" => SensitivityLevel::Public,
        f if f.starts_with("address[") => {
            if f.ends_with("].country") || f.ends_with("].title") || f.ends_with("].label") {
                SensitivityLevel::Public
            } else {
                SensitivityLevel::Sensitive
            }
        }
        _ => SensitivityLevel::Sensitive,
    }
}

// ============================================================================
// Rate Limiter
// ============================================================================

/// 字段访问频率限制器（10次/分钟/字段）
pub struct RateLimiter {
    /// plugin_id -> field -> (last_reset, count)
    counters: Mutex<HashMap<String, HashMap<String, (Instant, u32)>>>,
    max_per_minute: u32,
}

impl RateLimiter {
    pub fn new(max_per_minute: u32) -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            max_per_minute,
        }
    }

    /// 检查是否允许访问，同时增加计数
    pub fn check(&self, plugin_id: &str, field: &str) -> bool {
        let mut counters = self.counters.lock().unwrap();
        let plugin_map = counters.entry(plugin_id.to_string()).or_default();
        let entry = plugin_map
            .entry(field.to_string())
            .or_insert((Instant::now(), 0));

        // 每分钟重置计数器
        if entry.0.elapsed() > Duration::from_secs(60) {
            entry.0 = Instant::now();
            entry.1 = 0;
        }

        entry.1 += 1;
        entry.1 <= self.max_per_minute
    }
}

// ============================================================================
// Audit Log
// ============================================================================

/// 审计日志条目
#[derive(Clone, Debug)]
pub struct AuditEntry {
    pub plugin_id: String,
    pub session_id: String,
    pub timestamp: Instant,
    pub action: AuditAction,
}

#[derive(Clone, Debug)]
pub enum AuditAction {
    FieldAccessGranted {
        field: String,
        confirmed_by_user: bool,
    },
    FieldAccessDenied {
        field: String,
    },
    NetworkBlocked {
        url: String,
    },
    NetworkAllowed {
        url: String,
    },
    RateLimitTriggered {
        field: String,
    },
    PluginCrashed {
        reason: String,
    },
    SessionCreated,
    SessionRevoked,
}

fn log_audit(
    tx: &mpsc::Sender<AuditEntry>,
    plugin_id: &str,
    session_id: &str,
    action: AuditAction,
) {
    let _ = tx.try_send(AuditEntry {
        plugin_id: plugin_id.to_string(),
        session_id: session_id.to_string(),
        timestamp: Instant::now(),
        action,
    });
}

// ============================================================================
// SoloHostFunctions
// ============================================================================

/// Host functions state for a single plugin session
pub struct SoloHostFunctions {
    pub plugin_id: String,
    pub plugin_name: String,
    pub session_id: String,
    pub manifest: PluginManifest,
    pub consent_tx: mpsc::Sender<ConsentRequest>,
    pub audit_tx: mpsc::Sender<AuditEntry>,
    pub log_tx: mpsc::Sender<(String, String)>,
    /// Phase 2: 结构化结果通道
    pub result_tx: mpsc::Sender<String>,
    pub rate_limiter: Arc<RateLimiter>,
    pub session_expires_at: Instant,
    pub wasi: wasmtime_wasi::preview1::WasiP1Ctx,
    /// 预授权的字段集合（批量授权后填充）
    pub pre_approved_fields: HashSet<String>,
    /// Dart 端传入的初始参数（JSON 字符串）
    pub initial_params: Option<String>,
}

impl SoloHostFunctions {
    pub fn new(
        plugin_id: &str,
        plugin_name: &str,
        session_id: &str,
        manifest: PluginManifest,
        consent_tx: mpsc::Sender<ConsentRequest>,
        audit_tx: mpsc::Sender<AuditEntry>,
        log_tx: mpsc::Sender<(String, String)>,
        result_tx: mpsc::Sender<String>,
        rate_limiter: Arc<RateLimiter>,
        ttl_seconds: u64,
        pre_approved_fields: HashSet<String>,
        initial_params: Option<String>,
    ) -> Self {
        let wasi = wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .build_p1();
        Self {
            plugin_id: plugin_id.to_string(),
            plugin_name: plugin_name.to_string(),
            session_id: session_id.to_string(),
            manifest,
            consent_tx,
            audit_tx,
            log_tx,
            result_tx,
            rate_limiter,
            session_expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
            wasi,
            pre_approved_fields,
            initial_params,
        }
    }

    /// 注册所有 Host Functions 到 Linker
    pub fn register(linker: &mut Linker<Self>) -> Result<(), String> {
        // solosoul_request_field(field_id_ptr, field_id_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "env",
                "solosoul_request_field",
                |mut caller: Caller<'_, Self>,
                 field_id_ptr: i32,
                 field_id_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let field_id =
                        read_memory(&mut caller, field_id_ptr as usize, field_id_len as usize);
                    rust_log(&format!("[host:solosoul_request_field] field_id='{}' out_cap={}", field_id, out_cap));

                    // 使用独立作用域获取 funcs 数据，避免与后续 write_memory 的 mutable borrow 冲突
                    let (plugin_id, session_id, manifest, rate_limiter, consent_tx, audit_tx, plugin_name, session_expires_at, pre_approved_fields) = {
                        let funcs = caller.data();
                        (
                            funcs.plugin_id.clone(),
                            funcs.session_id.clone(),
                            funcs.manifest.clone(),
                            Arc::clone(&funcs.rate_limiter),
                            funcs.consent_tx.clone(),
                            funcs.audit_tx.clone(),
                            funcs.plugin_name.clone(),
                            funcs.session_expires_at,
                            funcs.pre_approved_fields.clone(),
                        )
                    };

                    // 1. 速率限制检查
                    if !rate_limiter.check(&plugin_id, &field_id) {
                        rust_log(&format!("[host:solosoul_request_field] RateLimited field='{}'", field_id));
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::RateLimitTriggered {
                                field: field_id.clone(),
                            },
                        );
                        return -8; // RateLimited
                    }

                    // 2. 校验字段是否在 manifest 声明范围内
                    let manifest_has_field = manifest.is_field_requested(&field_id);
                    rust_log(&format!("[host:solosoul_request_field] manifest.is_field_requested('{}') => {}", field_id, manifest_has_field));
                    rust_log(&format!("[host:solosoul_request_field] manifest.required_fields={:?}", manifest.required_fields));
                    rust_log(&format!("[host:solosoul_request_field] manifest.optional_fields={:?}", manifest.optional_fields));
                    if !manifest_has_field {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::FieldAccessDenied {
                                field: field_id.clone(),
                            },
                        );
                        rust_log(&format!("[host:solosoul_request_field] RETURN -1 PermissionDenied field='{}' not in manifest", field_id));
                        return -1; // PermissionDenied
                    }

                    // 3. 校验 Session 未过期
                    if Instant::now() > session_expires_at {
                        rust_log(&format!("[host:solosoul_request_field] RETURN -3 TtlExpired"));
                        return -3; // TtlExpired
                    }

                    // 4. 检查是否为预授权字段（批量授权模式，支持通配符匹配）
                    let is_pre_approved = pre_approved_fields.iter().any(|f| crate::plugin::manifest::PluginManifest::field_matches(f, &field_id));
                    rust_log(&format!("[host:solosoul_request_field] is_pre_approved={} pre_approved_fields={:?}", is_pre_approved, pre_approved_fields));
                    if is_pre_approved {
                        match decrypt_field_value(&field_id) {
                            Ok(value) => {
                                rust_log(&format!("[host:solosoul_request_field] pre_approved decrypt OK value_len={}", value.len()));
                                if value.len() + 1 > out_cap as usize {
                                    rust_log(&format!("[host:solosoul_request_field] RETURN -4 BufferTooSmall (value_len={} + null >= out_cap={})", value.len(), out_cap));
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, &value);
                                log_audit(
                                    &audit_tx,
                                    &plugin_id,
                                    &session_id,
                                    AuditAction::FieldAccessGranted {
                                        field: field_id,
                                        confirmed_by_user: true,
                                    },
                                );
                                rust_log(&format!("[host:solosoul_request_field] RETURN 0 pre_approved granted"));
                                return 0;
                            }
                            Err(e) => {
                                rust_log(&format!("[host:solosoul_request_field] pre_approved decrypt FAILED: {}", e));
                                return -5; // InvalidField
                            }
                        }
                    }

                    // 5. 根据敏感度决定是否需要用户确认
                    let sensitivity = resolve_field_sensitivity(&field_id);
                    let needs_confirmation = sensitivity.needs_confirmation();
                    rust_log(&format!("[host:solosoul_request_field] sensitivity={:?} needs_confirmation={}", sensitivity, needs_confirmation));

                    if !needs_confirmation {
                        match decrypt_field_value(&field_id) {
                            Ok(value) => {
                                rust_log(&format!("[host:solosoul_request_field] public/internal decrypt OK value_len={}", value.len()));
                                if value.len() + 1 > out_cap as usize {
                                    rust_log(&format!("[host:solosoul_request_field] RETURN -4 BufferTooSmall"));
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, &value);
                                log_audit(
                                    &audit_tx,
                                    &plugin_id,
                                    &session_id,
                                    AuditAction::FieldAccessGranted {
                                        field: field_id,
                                        confirmed_by_user: false,
                                    },
                                );
                                rust_log(&format!("[host:solosoul_request_field] RETURN 0 public/internal granted"));
                                return 0;
                            }
                            Err(e) => {
                                rust_log(&format!("[host:solosoul_request_field] public/internal decrypt FAILED: {}", e));
                                return -5; // InvalidField
                            }
                        }
                    }

                    // 5. 敏感字段：通过 Flutter 通道请求用户确认
                    let request_id = uuid::Uuid::new_v4().to_string();
                    let (tx, rx) = std::sync::mpsc::channel();
                    let request = ConsentRequest {
                        request_id: request_id.clone(),
                        plugin_id: plugin_id.clone(),
                        plugin_name: plugin_name.clone(),
                        field: field_id.clone(),
                        session_id: session_id.clone(),
                        sensitivity,
                        response: tx,
                    };

                    if consent_tx.try_send(request).is_err() {
                        return -1; // Consent channel closed
                    }

                    // 6. 阻塞等待 Flutter 用户响应（超时 60s）
                    // 使用 std::sync::mpsc::recv_timeout 防止 Dart 端因死锁或异常永远无法响应
                    {
                        use std::fs::OpenOptions;
                        use std::io::Write;
                        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("/tmp/solosoul_rust.log") {
                            let _ = writeln!(file, "[{}] request_field waiting consent: field={}, request_id={}", now, field_id, request_id);
                        }
                    }
                    match rx.recv_timeout(Duration::from_secs(60)) {
                        Ok(ConsentResult::Approved(value)) => {
                            rust_log(&format!("[host:solosoul_request_field] consent APPROVED value_len={}", value.len()));
                            if value.len() + 1 > out_cap as usize {
                                rust_log(&format!("[host:solosoul_request_field] RETURN -4 BufferTooSmall after consent"));
                                return -4;
                            }
                            write_memory(&mut caller, out_ptr as usize, &value);
                            log_audit(
                                &audit_tx,
                                &plugin_id,
                                &session_id,
                                AuditAction::FieldAccessGranted {
                                    field: field_id,
                                    confirmed_by_user: true,
                                },
                            );
                            rust_log(&format!("[host:solosoul_request_field] RETURN 0 consent granted"));
                            0
                        }
                        Ok(ConsentResult::Denied) => {
                            rust_log(&format!("[host:solosoul_request_field] RETURN -2 UserDenied"));
                            -2
                        }        // UserDenied
                        Ok(ConsentResult::Expired) => {
                            rust_log(&format!("[host:solosoul_request_field] RETURN -3 Expired"));
                            -3
                        }
                        Err(_) => {
                            rust_log(&format!("[host:solosoul_request_field] RETURN -3 Timeout/Disconnected"));
                            -3
                        } // Timeout / Disconnected
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_post_data(url_ptr, url_len, body_ptr, body_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "env",
                "solosoul_post_data",
                |mut caller: Caller<'_, Self>,
                 url_ptr: i32,
                 url_len: i32,
                 body_ptr: i32,
                 body_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let url = read_memory(&mut caller, url_ptr as usize, url_len as usize);
                    let body = read_memory(&mut caller, body_ptr as usize, body_len as usize);

                    let (plugin_id, session_id, manifest, audit_tx) = {
                        let funcs = caller.data();
                        (
                            funcs.plugin_id.clone(),
                            funcs.session_id.clone(),
                            funcs.manifest.clone(),
                            funcs.audit_tx.clone(),
                        )
                    };

                    if !is_network_allowed(&manifest, &url) {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::NetworkBlocked { url: url.clone() },
                        );
                        return -10; // DomainNotAllowed
                    }

                    // 执行 HTTP POST（在 tokio runtime 中阻塞执行）
                    let response = match block_on_http(proxy_http_post(&url, &body)) {
                        Ok(data) => {
                            log_audit(
                                &audit_tx,
                                &plugin_id,
                                &session_id,
                                AuditAction::NetworkAllowed { url: url.clone() },
                            );
                            Ok(data)
                        }
                        Err(e) => Err(e),
                    };

                    match response {
                        Ok(data) => {
                            if data.len() + 1 > out_cap as usize {
                                return -4; // BufferTooSmall
                            }
                            write_memory(&mut caller, out_ptr as usize, &data);
                            0
                        }
                        Err(HttpError::Timeout) => -6,
                        Err(_) => -1,
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_log(level_ptr, level_len, msg_ptr, msg_len)
        linker
            .func_wrap(
                "env",
                "solosoul_log",
                |mut caller: Caller<'_, Self>,
                 level_ptr: i32,
                 level_len: i32,
                 msg_ptr: i32,
                 msg_len: i32| {
                    let level = read_memory(&mut caller, level_ptr as usize, level_len as usize);
                    let message = read_memory(&mut caller, msg_ptr as usize, msg_len as usize);

                    let funcs = caller.data();
                    let _ = funcs.audit_tx.try_send(AuditEntry {
                        plugin_id: funcs.plugin_id.clone(),
                        session_id: funcs.session_id.clone(),
                        timestamp: Instant::now(),
                        action: AuditAction::NetworkAllowed {
                            url: format!("[LOG:{}] {}", level, message),
                        },
                    });
                    let _ = funcs.log_tx.try_send((level, message));
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_result(data_ptr, data_len) -> i32
        // Phase 2: 结构化结果通道 — 插件发送 JSON 数据，主软件按 type 渲染卡片
        linker
            .func_wrap(
                "env",
                "solosoul_result",
                |mut caller: Caller<'_, Self>,
                 data_ptr: i32,
                 data_len: i32|
                 -> i32 {
                    let data = read_memory(&mut caller, data_ptr as usize, data_len as usize);

                    // JSON 校验
                    const MAX_SIZE: usize = 64 * 1024; // 64KB
                    const MAX_DEPTH: usize = 10;
                    const VALID_TYPES: &[&str] = &["text", "key_value", "table", "map", "markdown", "calendar_events", "data_completeness"];

                    if data.len() > MAX_SIZE {
                        rust_log(&format!("[host:solosoul_result] rejected: size {} > {}", data.len(), MAX_SIZE));
                        return -1; // SizeExceeded
                    }

                    let json_str = data;

                    // 校验 JSON 格式和嵌套深度
                    match serde_json::from_str::<serde_json::Value>(&json_str) {
                        Ok(val) => {
                            if json_depth(&val) > MAX_DEPTH {
                                rust_log(&format!("[host:solosoul_result] rejected: depth > {}", MAX_DEPTH));
                                return -3; // DepthExceeded
                            }
                            // 校验 type 字段
                            match val.get("type").and_then(|v| v.as_str()) {
                                Some(t) if VALID_TYPES.contains(&t) => {}
                                Some(t) => {
                                    rust_log(&format!("[host:solosoul_result] rejected: invalid type '{}'", t));
                                    return -4; // InvalidType
                                }
                                None => {
                                    rust_log("[host:solosoul_result] rejected: missing 'type' field");
                                    return -5; // MissingType
                                }
                            }
                        }
                        Err(e) => {
                            rust_log(&format!("[host:solosoul_result] rejected: invalid JSON: {}", e));
                            return -6; // InvalidJson
                        }
                    }

                    let funcs = caller.data();
                    let _ = funcs.audit_tx.try_send(AuditEntry {
                        plugin_id: funcs.plugin_id.clone(),
                        session_id: funcs.session_id.clone(),
                        timestamp: Instant::now(),
                        action: AuditAction::NetworkAllowed {
                            url: format!("[RESULT] {} bytes", json_str.len()),
                        },
                    });
                    let _ = funcs.result_tx.try_send(json_str);
                    0 // Success
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_get_timestamp() -> i64
        linker
            .func_wrap(
                "env",
                "solosoul_get_timestamp",
                |_caller: Caller<'_, Self>| -> i64 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_get_data_structure_tree(out_ptr, out_cap) -> i32
        // Phase 3: 数据结构树查询 — 插件获取用户数据结构元数据（页面 → 分区 → 字段）
        linker
            .func_wrap(
                "env",
                "solosoul_get_data_structure_tree",
                |mut caller: Caller<'_, Self>,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    match build_data_structure_tree() {
                        Ok(json) => {
                            if json.len() + 1 > out_cap as usize {
                                rust_log(&format!("[host:solosoul_get_data_structure_tree] BufferTooSmall: need {} got {}", json.len() + 1, out_cap));
                                return -4; // BufferTooSmall
                            }
                            write_memory(&mut caller, out_ptr as usize, &json);
                            rust_log(&format!("[host:solosoul_get_data_structure_tree] OK, {} bytes", json.len()));
                            0 // Success
                        }
                        Err(e) => {
                            rust_log(&format!("[host:solosoul_get_data_structure_tree] error: {}", e));
                            -1 // Error
                        }
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_show_dialog(config_ptr, config_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "env",
                "solosoul_show_dialog",
                |mut caller: Caller<'_, Self>,
                 config_ptr: i32,
                 config_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let config = read_memory(&mut caller, config_ptr as usize, config_len as usize);
                    rust_log(&format!("[host:solosoul_show_dialog] config_len={}", config.len()));

                    let (plugin_id, plugin_name, session_id, consent_tx, log_tx) = {
                        let funcs = caller.data();
                        (
                            funcs.plugin_id.clone(),
                            funcs.plugin_name.clone(),
                            funcs.session_id.clone(),
                            funcs.consent_tx.clone(),
                            funcs.log_tx.clone(),
                        )
                    };

                    // 0. 安全白名单：仅允许官方插件调用对话框（防止滥用）
                    const ALLOWED_DIALOG_PLUGINS: &[&str] = &["com.solosoul.official.doc-checklist"];
                    if !ALLOWED_DIALOG_PLUGINS.contains(&plugin_id.as_str()) {
                        rust_log(&format!("[host:solosoul_show_dialog] RETURN -5 PermissionDenied plugin_id='{}'", plugin_id));
                        return -5; // PermissionDenied
                    }

                    let request_id = uuid::Uuid::new_v4().to_string();
                    let (tx, rx) = std::sync::mpsc::channel();

                    // 1. 通过 Log 事件传递对话框配置（Dart 端缓存）
                    // 发送 3 次并间隔 100ms，确保 Dart 端能收到（StreamSink 时序竞争保护）
                    let config_payload = format!("{}|{}", request_id, config);
                    for i in 0..3 {
                        if log_tx.try_send(("dialog_config".to_string(), config_payload.clone())).is_err() {
                            rust_log(&format!("[host:solosoul_show_dialog] log try_send failed at attempt {}", i));
                            return -1;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }

                    // 2. 通过 ConsentRequest 请求 Dart 弹出对话框（复用现有通道）
                    let request = ConsentRequest {
                        request_id: request_id.clone(),
                        plugin_id: plugin_id.clone(),
                        plugin_name: plugin_name.clone(),
                        field: "__dialog__".to_string(),
                        session_id: session_id.clone(),
                        sensitivity: SensitivityLevel::Public,
                        response: tx,
                    };

                    if consent_tx.try_send(request).is_err() {
                        rust_log("[host:solosoul_show_dialog] RETURN -1 Consent channel closed");
                        return -1; // Channel closed
                    }

                    // 3. 阻塞等待 Dart 用户响应（超时 60s）
                    match rx.recv_timeout(Duration::from_secs(60)) {
                        Ok(ConsentResult::Approved(result)) => {
                            rust_log(&format!("[host:solosoul_show_dialog] APPROVED result_len={}", result.len()));
                            if result.len() + 1 > out_cap as usize {
                                rust_log(&format!("[host:solosoul_show_dialog] RETURN -4 BufferTooSmall (need {} got {})", result.len() + 1, out_cap));
                                return -4; // BufferTooSmall
                            }
                            write_memory(&mut caller, out_ptr as usize, &result);
                            0 // Success
                        }
                        Ok(ConsentResult::Denied) => {
                            rust_log("[host:solosoul_show_dialog] RETURN -2 UserDenied");
                            -2 // User cancelled
                        }
                        Ok(ConsentResult::Expired) | Err(_) => {
                            rust_log("[host:solosoul_show_dialog] RETURN -3 Timeout");
                            -3 // Timeout
                        }
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_get_param(key_ptr, key_len, out_ptr, out_cap) -> i32
        // 读取 Dart 端传入的初始参数（JSON 字符串）中的指定 key
        linker
            .func_wrap(
                "env",
                "solosoul_get_param",
                |mut caller: Caller<'_, Self>,
                 key_ptr: i32,
                 key_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let key = read_memory(&mut caller, key_ptr as usize, key_len as usize);
                    rust_log(&format!("[host:solosoul_get_param] key='{}'", key));

                    let params_str = {
                        let funcs = caller.data();
                        funcs.initial_params.clone()
                    };

                    if let Some(ref params_str) = params_str {
                        if let Ok(params) = serde_json::from_str::<serde_json::Value>(params_str) {
                            if let Some(value) = params.get(&key).and_then(|v| v.as_str()) {
                                if value.len() + 1 > out_cap as usize {
                                    rust_log(&format!("[host:solosoul_get_param] RETURN -4 BufferTooSmall (need {} got {})", value.len() + 1, out_cap));
                                    return -4; // BufferTooSmall
                                }
                                write_memory(&mut caller, out_ptr as usize, value);
                                rust_log(&format!("[host:solosoul_get_param] RETURN 0 value='{}'", value));
                                return 0; // Success
                            }
                        }
                    }
                    rust_log("[host:solosoul_get_param] RETURN -5 NotFound");
                    -5 // NotFound
                },
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

fn is_network_allowed(manifest: &PluginManifest, url: &str) -> bool {
    let Some(ref policy) = manifest.network_policy else {
        return false; // 默认拒绝所有出站访问
    };
    if policy.block_all_outbound {
        return false; // 全部阻止，仅白名单例外
    }
    // 解析域名并匹配白名单
    let host = extract_host(url);
    policy.allows_domain(&host)
}

/// 从 Vault 中同步解密字段值
///
/// 执行流程：
/// 1. 获取 AccountManager → session_key + vault_store
/// 2. 加载第一个 Profile 的加密 data
/// 3. 用 session_key 解密（支持 SOLO blob 和 Legacy Dart 格式）
/// 4. 反序列化为 ProfileData，按字段路径提取值
/// 构建用户数据结构树（页面 → 分区 → 字段元数据）。
/// 供插件通过 `solosoul_get_data_structure_tree` Host Function 查询。
/// 返回 JSON 字符串，不包含任何字段值。
fn build_data_structure_tree() -> Result<String, String> {
    rust_log("[build_data_structure_tree] START");

    // 1. 获取 AccountManager
    let manager_guard = crate::get_account_manager()
        .map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    // 2. 获取 session_key
    let session_key = manager
        .get_session_key()
        .ok_or("Vault not unlocked")?;
    rust_log("[build_data_structure_tree] session_key obtained");

    // 3. 获取 vault_store
    let vault_guard = manager
        .get_vault_store()
        .ok_or("Vault store not available")?;
    let vault_store = vault_guard
        .as_ref()
        .ok_or("Vault store not open")?;

    // 4. 列出 profiles 并取第一个
    let profiles = vault_store
        .list_profiles()
        .map_err(|e| format!("Failed to list profiles: {}", e))?;
    rust_log(&format!("[build_data_structure_tree] profiles count={}", profiles.len()));
    let profile_summary = profiles.first().ok_or("No profiles found")?;

    // 5. 加载 profile 的加密 data
    let profile = vault_store
        .load_profile(&profile_summary.id)
        .map_err(|e| format!("Failed to load profile: {}", e))?
        .ok_or("Profile not found")?;

    // 6. 解密
    let plaintext = crate::crypto::decrypt_profile_data(&session_key, &profile.data)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    rust_log(&format!("[build_data_structure_tree] plaintext decrypted, len={}", plaintext.len()));

    // 7. 解析 JSON
    let json_str = String::from_utf8_lossy(&plaintext);
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse profile data: {}", e))?;

    // 8. 获取 unified_objects.objects
    let root = if json_value.get("data").is_some() && json_value.get("unified_objects").is_none() {
        json_value.get("data").ok_or("Missing data field")?
    } else {
        &json_value
    };
    let unified_objects = root.get("unified_objects")
        .ok_or("Missing unified_objects field")?;
    let objects = unified_objects.get("objects")
        .and_then(|o| o.as_array())
        .ok_or("Missing objects array")?;
    rust_log(&format!("[build_data_structure_tree] total objects count={}", objects.len()));

    // 9. 构建树结构
    let mut tree = Vec::new();

    // 找到所有页面（typeId == "page" 且 parentId == null）
    for obj in objects {
        let is_deleted = obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false);
        if is_deleted { continue; }

        let type_id = obj.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
        let parent_id = obj.get("parentId").and_then(|v| v.as_str());

        if type_id == "page" && parent_id.is_none() {
            let page_id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let page_name = obj.get("name").and_then(|v| v.as_str()).unwrap_or(page_id);

            let mut sections = Vec::new();

            // 找到该页面下的所有分区
            for child in objects {
                let child_deleted = child.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false);
                if child_deleted { continue; }

                let child_parent_id = child.get("parentId").and_then(|v| v.as_str());
                let child_type_id = child.get("typeId").and_then(|v| v.as_str()).unwrap_or("");

                if child_parent_id == Some(page_id) && child_type_id != "page" {
                    let section_id = child.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let section_name = child.get("name").and_then(|v| v.as_str()).unwrap_or(section_id);
                    let section_type_id = child_type_id;

                    let mut fields = Vec::new();

                    // 从 properties 中提取字段（仅元数据：id、name、type，不包含值）
                    if let Some(props) = child.get("properties").and_then(|p| p.as_object()) {
                        let prop_labels = child.get("propertyLabels")
                            .and_then(|p| p.as_object())
                            .map(|m| m.clone())
                            .unwrap_or_default();

                        for (key, value) in props {
                            let field_type = value.get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            let label = prop_labels.get(key)
                                .and_then(|v| v.as_str())
                                .unwrap_or(key);

                            fields.push(serde_json::json!({
                                "id": key,
                                "name": label,
                                "fieldType": field_type,
                            }));
                        }
                    }

                    sections.push(serde_json::json!({
                        "id": section_id,
                        "type": "section",
                        "name": section_name,
                        "typeId": section_type_id,
                        "fields": fields,
                    }));
                }
            }

            tree.push(serde_json::json!({
                "id": page_id,
                "type": "page",
                "name": page_name,
                "sections": sections,
            }));
        }
    }

    let result = serde_json::to_string(&tree)
        .map_err(|e| format!("Failed to serialize tree: {}", e))?;
    rust_log(&format!("[build_data_structure_tree] OK, {} bytes", result.len()));
    Ok(result)
}

fn decrypt_field_value(field_id: &str) -> Result<String, String> {
    rust_log(&format!("[decrypt_field_value] START field_id={}", field_id));

    // 1. 获取 AccountManager
    let manager_guard = crate::get_account_manager()
        .map_err(|e| format!("Account manager error: {}", e))?;
    let manager = manager_guard
        .as_ref()
        .ok_or("Account manager not initialized")?;

    // 2. 获取 session_key
    let session_key = manager
        .get_session_key()
        .ok_or("Vault not unlocked")?;
    rust_log("[decrypt_field_value] session_key obtained");

    // 3. 获取 vault_store
    let vault_guard = manager
        .get_vault_store()
        .ok_or("Vault store not available")?;
    let vault_store = vault_guard
        .as_ref()
        .ok_or("Vault store not open")?;

    // 4. 列出 profiles 并取第一个
    let profiles = vault_store
        .list_profiles()
        .map_err(|e| format!("Failed to list profiles: {}", e))?;
    rust_log(&format!("[decrypt_field_value] profiles count={}", profiles.len()));
    let profile_summary = profiles.first().ok_or("No profiles found")?;
    rust_log(&format!("[decrypt_field_value] using profile_id={}", profile_summary.id));

    // 5. 加载 profile 的加密 data
    let profile = vault_store
        .load_profile(&profile_summary.id)
        .map_err(|e| format!("Failed to load profile: {}", e))?
        .ok_or("Profile not found")?;

    // 6. 解密
    let plaintext = crate::crypto::decrypt_profile_data(&session_key, &profile.data)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    let plaintext_len = plaintext.len();
    rust_log(&format!("[decrypt_field_value] plaintext decrypted, len={}", plaintext_len));

    // 7. 反序列化（支持 VersionedProfileData 和旧格式 ProfileData）
    let json_str = String::from_utf8_lossy(&plaintext);
    let is_versioned = json_str.trim_start().starts_with('{') && json_str.contains("\"version\"");
    rust_log(&format!("[decrypt_field_value] json_str prefix={}", &json_str[..json_str.len().min(200)]));
    let data: ProfileData = match serde_json::from_str::<VersionedProfileData>(&json_str) {
        Ok(versioned) => {
            rust_log(&format!("[decrypt_field_value] parsed as VersionedProfileData, version={}", versioned.version));
            versioned.data
        }
        Err(e) => {
            rust_log(&format!("[decrypt_field_value] NOT VersionedProfileData: {}, trying direct ProfileData", e));
            // 向后兼容：旧格式没有 version 包装层
            serde_json::from_str(&json_str)
                .map_err(|e| format!("Failed to parse profile data: {}", e))?
        }
    };

    // 8. 按字段路径取值（旧格式）
    let result = extract_field_value(field_id, &data);
    rust_log(&format!("[decrypt_field_value] legacy extract_field_value result={:?}", result.as_ref().map(|s| &s[..s.len().min(50)])));
    if result.is_some() {
        rust_log(&format!("[decrypt_field_value] RETURN legacy result"));
        return result.ok_or_else(|| format!("Field '{}' not found or empty", field_id));
    }

    // 9. 尝试从 Unified Object Model 提取（Flutter 新格式）
    let json_value: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse as JSON: {}", e))?;
    rust_log(&format!("[decrypt_field_value] parsed as serde_json::Value, has_data={}, has_unified_objects={}",
        json_value.get("data").is_some(), json_value.get("unified_objects").is_some()));

    // 【新增】语义类型路径分支（semantic://）
    if field_id.contains("semantic://") {
        rust_log(&format!("[decrypt_field_value] semantic path detected: {}", field_id));
        let semantic_result = extract_by_semantic_type(field_id, &json_value, None);
        rust_log(&format!("[decrypt_field_value] semantic result={:?}", semantic_result.as_ref().map(|s| &s[..s.len().min(50)])));
        return semantic_result.ok_or_else(|| format!("Semantic field '{}' not found or empty", field_id));
    }

    let uom_result = extract_from_unified_object_model(field_id, &json_value);
    rust_log(&format!("[decrypt_field_value] UOM result={:?}", uom_result.as_ref().map(|s| &s[..s.len().min(100)])));
    if let Some(ref val) = uom_result {
        rust_log(&format!("[decrypt_field_value] RETURN UOM field_id='{}' value_len={} value='{}'", field_id, val.len(), val));
    } else {
        rust_log(&format!("[decrypt_field_value] UOM returned None, will return Err for field_id='{}'", field_id));
    }
    uom_result.ok_or_else(|| format!("Field '{}' not found or empty", field_id))
}

/// 从 ProfileData 中按字段路径提取值
fn extract_field_value(field_id: &str, data: &ProfileData) -> Option<String> {
    rust_log(&format!("[extract_field_value] START field_id={}", field_id));
    // 支持 address 数组索引语法，如 address[0].street
    if let Some(addr_field) = field_id.strip_prefix("address[") {
        if let Some((idx_str, rest)) = addr_field.split_once("].") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                let identity = match data.identity.as_ref() {
                    Some(i) => i,
                    None => {
                        rust_log("[extract_field_value] data.identity is None");
                        return None;
                    }
                };
                let addrs: Vec<_> = identity.addresses.iter().filter(|a| !a.is_deleted).collect();
                rust_log(&format!("[extract_field_value] legacy addrs count={}", addrs.len()));
                let addr = match addrs.get(idx) {
                    Some(a) => a,
                    None => {
                        rust_log(&format!("[extract_field_value] legacy addrs[{}] not found", idx));
                        return None;
                    }
                };
                let result = match rest {
                    "street" => addr.street.clone(),
                    "city" => addr.city.clone(),
                    "state" => addr.state.clone(),
                    "district" => addr.district.clone(),
                    "postalCode" => addr.postal_code.clone(),
                    "country" => addr.country.clone(),
                    "title" | "label" => addr.label.clone(),
                    _ => None,
                };
                rust_log(&format!("[extract_field_value] legacy address[{}].{} => {:?}", idx, rest, result.as_ref().map(|s| &s[..s.len().min(50)])));
                return result;
            }
        }
    }

    // address.count 返回未删除地址数量
    if field_id == "address.count" {
        let count = data.identity.as_ref()?.addresses.iter().filter(|a| !a.is_deleted).count();
        rust_log(&format!("[extract_field_value] legacy address.count => {}", count));
        return Some(count.to_string());
    }

    // address.xxx 简写路径：默认映射到第一个未删除地址（主地址）
    if let Some(addr_key) = field_id.strip_prefix("address.") {
        let identity = match data.identity.as_ref() {
            Some(i) => i,
            None => {
                rust_log("[extract_field_value] data.identity is None for shorthand");
                return None;
            }
        };
        let addr = match identity.addresses.iter().find(|a| !a.is_deleted) {
            Some(a) => a,
            None => {
                rust_log("[extract_field_value] legacy no non-deleted address found");
                return None;
            }
        };
        let result = match addr_key {
            "street" => addr.street.clone(),
            "city" => addr.city.clone(),
            "state" => addr.state.clone(),
            "district" => addr.district.clone(),
            "postalCode" => addr.postal_code.clone(),
            "country" => addr.country.clone(),
            "title" | "label" => addr.label.clone(),
            _ => None,
        };
        rust_log(&format!("[extract_field_value] legacy address.{} => {:?}", addr_key, result.as_ref().map(|s| &s[..s.len().min(50)])));
        return result;
    }

    match field_id {
        "identity.full_name" => data.identity.as_ref()?.full_name.clone(),
        "identity.contact.emails" => {
            let emails: Vec<String> = data
                .identity
                .as_ref()?
                .contact
                .as_ref()?
                .entries
                .iter()
                .filter(|e| e.entry_type == "email")
                .map(|e| e.value.clone())
                .collect();
            if emails.is_empty() {
                None
            } else {
                Some(emails.join(", "))
            }
        }
        "identity.contact.phones" => {
            let phones: Vec<String> = data
                .identity
                .as_ref()?
                .contact
                .as_ref()?
                .entries
                .iter()
                .filter(|e| e.entry_type == "phone")
                .map(|e| e.value.clone())
                .collect();
            if phones.is_empty() {
                None
            } else {
                Some(phones.join(", "))
            }
        }
        "identity.id_card.number" => data
            .identity
            .as_ref()?
            .id_cards
            .iter()
            .find(|c| !c.is_deleted)
            .and_then(|c| c.number.clone()),
        "travel.primary_passport.number" => data
            .travel
            .as_ref()?
            .passports
            .iter()
            .find(|p| !p.is_deleted)
            .and_then(|p| p.number.clone()),
        "financial.primary_bank_account.number" => data
            .financial
            .as_ref()?
            .bank_accounts
            .iter()
            .find(|b| !b.is_deleted)
            .and_then(|b| b.account_number.clone()),
        _ => None,
    }
}

/// 从 Unified Object Model（Flutter 新格式）中提取字段值
///
/// UOM JSON 结构：
/// {
///   "unified_objects": {
///     "objects": [
///       {
///         "name": "Home Address",
///         "properties": {
///           "street": {"type": "text", "text": "...", "sensitivity": "..."},
///           ...
///         }
///       }
///     ]
///   }
/// }
/// 获取所有非删除的 profile_address 对象（按数组索引顺序）
fn get_address_objects(objects: &[serde_json::Value]) -> Vec<&serde_json::Value> {
    rust_log(&format!("[get_address_objects] input objects count={}", objects.len()));
    let mut addrs: Vec<&serde_json::Value> = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        let is_deleted = obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false);
        let type_id = obj.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
        rust_log(&format!("[get_address_objects] obj[{}] typeId={} isDeleted={}", i, type_id, is_deleted));
        if is_deleted {
            continue;
        }
        // 支持旧版 profile_address 和新版 __preset_address
        if type_id == "profile_address" || type_id == "__preset_address" {
            addrs.push(obj);
        }
    }
    rust_log(&format!("[get_address_objects] filtered address count={}", addrs.len()));
    addrs
}

fn extract_from_unified_object_model(field_id: &str, json_value: &serde_json::Value) -> Option<String> {
    rust_log(&format!("[extract_from_uom] START field_id={}", field_id));
    // 支持两种 JSON 结构：
    // 1. VersionedProfileData 包装层: { "version": 1, "data": { "unified_objects": { "objects": [...] } } }
    // 2. 直接 ProfileData: { "unified_objects": { "objects": [...] } }
    let root = if json_value.get("data").is_some() && json_value.get("unified_objects").is_none() {
        rust_log("[extract_from_uom] using json_value.data (VersionedProfileData wrapper)");
        json_value.get("data")?
    } else {
        rust_log("[extract_from_uom] using json_value directly (ProfileData)");
        json_value
    };
    let unified_objects = root.get("unified_objects")?;
    let objects = unified_objects.get("objects")?.as_array()?;
    rust_log(&format!("[extract_from_uom] total objects count={}", objects.len()));

    // 1. 处理 address.count
    if field_id == "address.count" {
        let addrs = get_address_objects(objects);
        rust_log(&format!("[extract_from_uom] address.count => {}", addrs.len()));
        return Some(addrs.len().to_string());
    }

    // 2. 处理 address[N].xxx 数组索引语法
    if let Some(addr_field) = field_id.strip_prefix("address[") {
        rust_log(&format!("[extract_from_uom] matched address[N].xxx syntax, addr_field={}", addr_field));
        if let Some((idx_str, rest)) = addr_field.split_once("].") {
            rust_log(&format!("[extract_from_uom] idx_str={}, rest={}", idx_str, rest));
            if let Ok(idx) = idx_str.parse::<usize>() {
                let addrs = get_address_objects(objects);
                rust_log(&format!("[extract_from_uom] looking up addrs[{}], total_addrs={}", idx, addrs.len()));
                let addr = addrs.get(idx)?;
                let properties = addr.get("properties")?;
                let prop_key = match rest {
                    "street" => "street",
                    "city" => "city",
                    "state" => "state",
                    "postalCode" => "postalCode",
                    "country" => "country",
                    "district" => "district",
                    // UOM 中地址对象的标签字段 id 是 "Title"（大写 T）
                    "title" | "label" => "Title",
                    _ => rest,
                };
                rust_log(&format!("[extract_from_uom] looking for prop_key='{}' in properties", prop_key));
                // 打印所有 properties key 用于调试
                if let Some(props_map) = properties.as_object() {
                    let keys: Vec<String> = props_map.keys().cloned().collect();
                    rust_log(&format!("[extract_from_uom] available property keys: {:?}", keys));
                }
                // 支持大小写回退：优先 "Title"，其次 "title"，最后回退到 name 字段
                let prop = properties.get(prop_key).or_else(|| {
                    if prop_key == "Title" {
                        properties.get("title")
                    } else {
                        None
                    }
                });
                let result = match prop {
                    Some(p) => {
                        let text = p.get("text").and_then(|t| t.as_str()).map(|s| s.to_string());
                        rust_log(&format!("[extract_from_uom] found prop_key='{}' text={:?}", prop_key, text.as_ref().map(|s| &s[..s.len().min(50)])));
                        text
                    }
                    None if prop_key == "Title" => {
                        let name_fallback = addr.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
                        rust_log(&format!("[extract_from_uom] prop_key='Title' not found in properties, falling back to name={:?}", name_fallback.as_ref().map(|s| &s[..s.len().min(50)])));
                        name_fallback
                    }
                    None => {
                        rust_log(&format!("[extract_from_uom] prop_key='{}' not found in properties", prop_key));
                        None
                    }
                };
                return result;
            } else {
                rust_log(&format!("[extract_from_uom] failed to parse idx_str='{}' as usize", idx_str));
            }
        }
    }

    // 3. 处理 address.xxx 简写路径（返回第一个非空地址的对应字段）
    if let Some(addr_key) = field_id.strip_prefix("address.") {
        rust_log(&format!("[extract_from_uom] matched address.xxx shorthand, addr_key={}", addr_key));
        let prop_key = match addr_key {
            "street" => "street",
            "city" => "city",
            "state" => "state",
            "postalCode" => "postalCode",
            "country" => "country",
            "district" => "district",
            // UOM 中地址对象的标签字段 id 是 "Title"（大写 T）
            "title" | "label" => "Title",
            _ => addr_key,
        };
        let addrs = get_address_objects(objects);
        let mut first_empty: Option<String> = None;
        for (i, addr) in addrs.iter().enumerate() {
            let properties = match addr.get("properties") {
                Some(p) => p,
                None => {
                    rust_log(&format!("[extract_from_uom] addr[{}] has no properties", i));
                    continue;
                }
            };
            // 支持大小写回退：优先 prop_key，其次小写变体，最后回退到 name 字段
            let prop_value = properties.get(prop_key).or_else(|| {
                if prop_key == "Title" {
                    properties.get("title")
                } else {
                    None
                }
            });
            let text_value = match prop_value {
                Some(p) => p.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()),
                None if prop_key == "Title" => {
                    let name_fallback = addr.get("name").and_then(|n| n.as_str()).map(|s| s.to_string());
                    if name_fallback.is_some() {
                        rust_log(&format!("[extract_from_uom] addr[{}] prop_key='Title' not found, falling back to name={:?}", i, name_fallback.as_ref().map(|s| &s[..s.len().min(50)])));
                    }
                    name_fallback
                }
                None => {
                    rust_log(&format!("[extract_from_uom] addr[{}] missing prop_key='{}'", i, prop_key));
                    continue;
                }
            };
            if let Some(text) = text_value {
                rust_log(&format!("[extract_from_uom] addr[{}] text='{}' (empty={})", i, &text[..text.len().min(50)], text.is_empty()));
                if !text.is_empty() {
                    return Some(text.to_string());
                }
                if first_empty.is_none() {
                    first_empty = Some(text.to_string());
                }
            } else {
                rust_log(&format!("[extract_from_uom] addr[{}] prop has no 'text' field", i));
            }
        }
        rust_log(&format!("[extract_from_uom] shorthand returning first_empty={:?}", first_empty.as_ref().map(|s| &s[..s.len().min(50)])));
        return first_empty;
    }

    // 4. 通用字段路径映射（identity / passport / visa / idCard / card / contact 等）
    // 解析 field_id 为 (type_filter, property_key)
    // 示例: "passport.expiryDate" -> ("__preset_passport" or "profile_passport", "expiryDate")
    let (type_filter, property_key) = if let Some((prefix, key)) = field_id.split_once('.') {
        let filter = match prefix {
            "passport" => Some("__preset_passport"),
            "visa" => Some("__preset_visa"),
            "idCard" => Some("__preset_idcard"),
            "card" => Some("__preset_card"),
            "identity" => Some("__preset_identity"),
            "contact" => Some("__preset_contact"),
            "travel" => Some("__preset_travel"),
            "financial" => Some("__preset_financial"),
            "professional" => Some("__preset_professional"),
            "health" => Some("__preset_health"),
            "education" => Some("__preset_education"),
            "property" => Some("__preset_property"),
            "vehicle" => Some("__preset_vehicle"),
            "pet" => Some("__preset_pet"),
            "insurance" => Some("__preset_insurance"),
            "membership" => Some("__preset_membership"),
            "subscription" => Some("__preset_subscription"),
            _ => None,
        };
        (filter, key)
    } else {
        (None, field_id)
    };
    rust_log(&format!("[extract_from_uom] generic field lookup, type_filter={:?} property_key='{}'", type_filter, property_key));

    let mut first_empty: Option<String> = None;
    for obj in objects {
        if obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        let type_id = obj.get("typeId").and_then(|v| v.as_str()).unwrap_or("");
        if type_id == "page" || type_id == "collection" {
            continue;
        }
        // 如果指定了 type_filter，只匹配对应类型的对象
        if let Some(filter) = type_filter {
            if type_id != filter && !type_id.ends_with(filter.strip_prefix("__preset_").unwrap_or(filter)) {
                continue;
            }
        }
        let properties = match obj.get("properties") {
            Some(p) => p,
            None => continue,
        };

        // 【新增】Passport 字段别名映射：surname / givenNames → holderName 拆分
        // 优先级：1) 精确匹配字段  2) holderName / holder_name 拆分
        if type_filter == Some("__preset_passport") {
            if property_key == "surname" {
                // 1) 尝试精确字段（多种命名风格，向前兼容）
                for exact_key in &["surname", "family_name", "familyName"] {
                    if let Some(p) = properties.get(exact_key) {
                        rust_log(&format!("[extract_from_uom] passport.surname found exact key='{}'", exact_key));
                        if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() { return Some(text.to_string()); }
                            if first_empty.is_none() { first_empty = Some(text.to_string()); }
                        }
                    }
                }
                // 2) fallback: 从 holderName / holder_name 拆分，取第一个词作为姓氏
                for holder_key in &["holderName", "holder_name"] {
                    if let Some(p) = properties.get(holder_key) {
                        if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                            let trimmed = text.trim();
                            let surname = trimmed.split_whitespace().next().unwrap_or("");
                            rust_log(&format!("[extract_from_uom] passport.surname split from '{}' => '{}'", holder_key, surname));
                            if !surname.is_empty() { return Some(surname.to_string()); }
                            if first_empty.is_none() { first_empty = Some(surname.to_string()); }
                        }
                    }
                }
                continue;
            }
            if property_key == "givenNames" || property_key == "given_name" {
                // 1) 尝试精确字段（多种命名风格）
                for exact_key in &["givenNames", "given_name", "givenName"] {
                    if let Some(p) = properties.get(exact_key) {
                        rust_log(&format!("[extract_from_uom] passport.givenNames found exact key='{}'", exact_key));
                        if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() { return Some(text.to_string()); }
                            if first_empty.is_none() { first_empty = Some(text.to_string()); }
                        }
                    }
                }
                // 2) fallback: 从 holderName / holder_name 拆分，跳过第一个词作为名字
                for holder_key in &["holderName", "holder_name"] {
                    if let Some(p) = properties.get(holder_key) {
                        if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                            let trimmed = text.trim();
                            let given = trimmed.split_whitespace().skip(1).collect::<Vec<_>>().join(" ");
                            rust_log(&format!("[extract_from_uom] passport.givenNames split from '{}' => '{}'", holder_key, given));
                            // givenNames 允许为空（单名情况），返回空字符串而非 None
                            return Some(given.to_string());
                        }
                    }
                }
                continue;
            }
            if property_key == "nationality" {
                // 1) 尝试 nationality 精确字段
                if let Some(p) = properties.get("nationality") {
                    if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                        let normalized = normalize_country_code(text);
                        rust_log(&format!("[extract_from_uom] passport.nationality raw='{}' normalized='{}'", text, normalized));
                        if !normalized.is_empty() {
                            return Some(normalized);
                        }
                    }
                }
                // 2) fallback: country / countryCode / country_code
                for fb_key in &["country", "countryCode", "country_code"] {
                    if let Some(p) = properties.get(fb_key) {
                        if let Some(text) = p.get("text").and_then(|t| t.as_str()) {
                            let normalized = normalize_country_code(text);
                            rust_log(&format!("[extract_from_uom] passport.nationality fallback from '{}' raw='{}' normalized='{}'", fb_key, text, normalized));
                            if !normalized.is_empty() {
                                return Some(normalized);
                            }
                        }
                    }
                }
                continue;
            }
        }

        // 【新增】Contact KV 数组智能提取：contact.email / contact.phone
        // UOM contact 对象是 KV 结构：{ type: "email", value: "a@b.com" }
        if type_filter == Some("__preset_contact") {
            let target_type = match property_key {
                "email" => "email",
                "phone" => "phone",
                _ => "",
            };
            if !target_type.is_empty() {
                if let Some(type_prop) = properties.get("type") {
                    if let Some(type_text) = type_prop.get("text").and_then(|t| t.as_str()) {
                        if type_text.eq_ignore_ascii_case(target_type) {
                            if let Some(value_prop) = properties.get("value") {
                                if let Some(value_text) = value_prop.get("text").and_then(|t| t.as_str()) {
                                    if !value_text.is_empty() {
                                        rust_log(&format!("[extract_from_uom] contact.{} matched type='{}' value='{}'", property_key, type_text, value_text));
                                        return Some(value_text.to_string());
                                    }
                                    if first_empty.is_none() {
                                        first_empty = Some(value_text.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                continue;
            }
        }

        // 【新增】通用字段别名映射（向后兼容旧插件路径）
        let effective_key = match (type_filter, property_key) {
            (Some("__preset_identity"), "fullName") => "full_name",
            (Some("__preset_identity"), "dateOfBirth") => "date_of_birth",
            (Some("__preset_identity"), "sex") => "gender",
            (Some("__preset_passport"), "placeOfBirth") => "place_of_birth",
            (Some("__preset_passport"), "issuingAuthority") => "place_of_issue",
            (Some("__preset_payment_card"), "number") => "card_number",
            _ => property_key,
        };
        if effective_key != property_key {
            rust_log(&format!("[extract_from_uom] alias mapping: '{}' -> '{}'", property_key, effective_key));
        }

        let prop = match properties.get(effective_key) {
            Some(p) => p,
            None => continue,
        };
        if let Some(text) = prop.get("text").and_then(|t| t.as_str()) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(text.to_string());
            }
            continue;
        }
        if let Some(value) = prop.get("value").and_then(|v| v.as_f64()) {
            return Some(value.to_string());
        }
        if let Some(iso_date) = prop.get("isoDate").and_then(|d| d.as_str()) {
            if !iso_date.is_empty() {
                return Some(iso_date.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(iso_date.to_string());
            }
            continue;
        }
        if let Some(checked) = prop.get("checked").and_then(|c| c.as_bool()) {
            return Some(checked.to_string());
        }
        if let Some(selected_id) = prop.get("selectedId").and_then(|s| s.as_str()) {
            if !selected_id.is_empty() {
                return Some(selected_id.to_string());
            }
            if first_empty.is_none() {
                first_empty = Some(selected_id.to_string());
            }
            continue;
        }
        if let Some(selected_ids) = prop.get("selectedIds").and_then(|s| s.as_array()) {
            let ids: Vec<String> = selected_ids.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
            let joined = ids.join(", ");
            if !joined.is_empty() {
                return Some(joined);
            }
            if first_empty.is_none() {
                first_empty = Some(joined);
            }
            continue;
        }
    }

    rust_log(&format!("[extract_from_uom] END returning first_empty={:?}", first_empty.as_ref().map(|s| &s[..s.len().min(50)])));
    first_empty
}

/// 将中文/英文国家名称转换为 ISO 3166-1 alpha-3 三位字母代码
/// 如果输入已经是3位纯字母代码，直接返回大写形式
fn normalize_country_code(raw: &str) -> String {
    let trimmed = raw.trim();
    let upper = trimmed.to_uppercase();
    // 如果已经是3位纯字母，直接返回大写
    if trimmed.len() == 3 && trimmed.chars().all(|c| c.is_ascii_alphabetic()) {
        return upper;
    }
    // 中文/英文映射表（覆盖主要国家和地区）
    let code: &str = match trimmed {
        "中国" | "中华人民共和国" | "CHINA" | "PEOPLE'S REPUBLIC OF CHINA" => "CHN",
        "美国" | "美利坚合众国" | "UNITED STATES" | "UNITED STATES OF AMERICA" | "AMERICA" => "USA",
        "英国" | "大不列颠及北爱尔兰联合王国" | "UNITED KINGDOM" | "GREAT BRITAIN" | "BRITAIN" => "GBR",
        "日本" | "JAPAN" => "JPN",
        "韩国" | "大韩民国" | "SOUTH KOREA" | "REPUBLIC OF KOREA" | "KOREA" => "KOR",
        "朝鲜" | "朝鲜民主主义人民共和国" | "NORTH KOREA" | "DPRK" | "DEMOCRATIC PEOPLE'S REPUBLIC OF KOREA" => "PRK",
        "加拿大" | "CANADA" => "CAN",
        "澳大利亚" | "澳洲" | "AUSTRALIA" => "AUS",
        "新西兰" | "NEW ZEALAND" => "NZL",
        "法国" | "法兰西共和国" | "FRANCE" => "FRA",
        "德国" | "德意志联邦共和国" | "GERMANY" | "DEUTSCHLAND" | "FEDERAL REPUBLIC OF GERMANY" => "DEU",
        "俄罗斯" | "俄罗斯联邦" | "RUSSIA" | "RUSSIAN FEDERATION" => "RUS",
        "意大利" | "ITALY" => "ITA",
        "西班牙" | "SPAIN" => "ESP",
        "葡萄牙" | "PORTUGAL" => "PRT",
        "荷兰" | "NETHERLANDS" | "HOLLAND" => "NLD",
        "比利时" | "BELGIUM" => "BEL",
        "瑞士" | "SWITZERLAND" => "CHE",
        "瑞典" | "SWEDEN" => "SWE",
        "挪威" | "NORWAY" => "NOR",
        "丹麦" | "DENMARK" => "DNK",
        "芬兰" | "FINLAND" => "FIN",
        "奥地利" | "AUSTRIA" => "AUT",
        "爱尔兰" | "IRELAND" => "IRL",
        "波兰" | "POLAND" => "POL",
        "捷克" | "CZECH REPUBLIC" | "CZECHIA" => "CZE",
        "匈牙利" | "HUNGARY" => "HUN",
        "希腊" | "GREECE" => "GRC",
        "罗马尼亚" | "ROMANIA" => "ROU",
        "保加利亚" | "BULGARIA" => "BGR",
        "乌克兰" | "UKRAINE" => "UKR",
        "塞尔维亚" | "SERBIA" => "SRB",
        "克罗地亚" | "CROATIA" => "HRV",
        "斯洛伐克" | "SLOVAKIA" => "SVK",
        "斯洛文尼亚" | "SLOVENIA" => "SVN",
        "白俄罗斯" | "BELARUS" => "BLR",
        "立陶宛" | "LITHUANIA" => "LTU",
        "拉脱维亚" | "LATVIA" => "LVA",
        "爱沙尼亚" | "ESTONIA" => "EST",
        "摩尔多瓦" | "MOLDOVA" => "MDA",
        "冰岛" | "ICELAND" => "ISL",
        "卢森堡" | "LUXEMBOURG" => "LUX",
        "马耳他" | "MALTA" => "MLT",
        "塞浦路斯" | "CYPRUS" => "CYP",
        "土耳其" | "TURKEY" | "TÜRKIYE" => "TUR",
        "新加坡" | "SINGAPORE" => "SGP",
        "马来西亚" | "MALAYSIA" => "MYS",
        "泰国" | "THAILAND" => "THA",
        "越南" | "VIETNAM" | "VIET NAM" => "VNM",
        "印度尼西亚" | "印尼" | "INDONESIA" => "IDN",
        "菲律宾" | "PHILIPPINES" => "PHL",
        "缅甸" | "MYANMAR" | "BURMA" => "MMR",
        "柬埔寨" | "CAMBODIA" => "KHM",
        "老挝" | "LAOS" => "LAO",
        "文莱" | "BRUNEI" => "BRN",
        "东帝汶" | "TIMOR-LESTE" | "EAST TIMOR" => "TLS",
        "印度" | "INDIA" => "IND",
        "巴基斯坦" | "PAKISTAN" => "PAK",
        "孟加拉国" | "孟加拉" | "BANGLADESH" => "BGD",
        "斯里兰卡" | "SRI LANKA" => "LKA",
        "尼泊尔" | "NEPAL" => "NPL",
        "不丹" | "BHUTAN" => "BTN",
        "马尔代夫" | "MALDIVES" => "MDV",
        "蒙古" | "MONGOLIA" => "MNG",
        "阿富汗" | "AFGHANISTAN" => "AFG",
        "伊朗" | "IRAN" => "IRN",
        "伊拉克" | "IRAQ" => "IRQ",
        "叙利亚" | "SYRIA" => "SYR",
        "约旦" | "JORDAN" => "JOR",
        "黎巴嫩" | "LEBANON" => "LBN",
        "以色列" | "ISRAEL" => "ISR",
        "巴勒斯坦" | "PALESTINE" | "STATE OF PALESTINE" => "PSE",
        "沙特阿拉伯" | "沙特" | "SAUDI ARABIA" => "SAU",
        "阿联酋" | "阿拉伯联合酋长国" | "UNITED ARAB EMIRATES" => "ARE",
        "科威特" | "KUWAIT" => "KWT",
        "卡塔尔" | "QATAR" => "QAT",
        "巴林" | "BAHRAIN" => "BHR",
        "阿曼" | "OMAN" => "OMN",
        "也门" | "YEMEN" => "YEM",
        "埃及" | "EGYPT" => "EGY",
        "利比亚" | "LIBYA" => "LBY",
        "突尼斯" | "TUNISIA" => "TUN",
        "阿尔及利亚" | "ALGERIA" => "DZA",
        "摩洛哥" | "MOROCCO" => "MAR",
        "苏丹" | "SUDAN" => "SDN",
        "南苏丹" | "SOUTH SUDAN" => "SSD",
        "埃塞俄比亚" | "ETHIOPIA" => "ETH",
        "厄立特里亚" | "ERITREA" => "ERI",
        "索马里" | "SOMALIA" => "SOM",
        "肯尼亚" | "KENYA" => "KEN",
        "坦桑尼亚" | "TANZANIA" => "TZA",
        "乌干达" | "UGANDA" => "UGA",
        "卢旺达" | "RWANDA" => "RWA",
        "布隆迪" | "BURUNDI" => "BDI",
        "赞比亚" | "ZAMBIA" => "ZMB",
        "津巴布韦" | "ZIMBABWE" => "ZWE",
        "博茨瓦纳" | "BOTSWANA" => "BWA",
        "纳米比亚" | "NAMIBIA" => "NAM",
        "莫桑比克" | "MOZAMBIQUE" => "MOZ",
        "马拉维" | "MALAWI" => "MWI",
        "马达加斯加" | "MADAGASCAR" => "MDG",
        "毛里求斯" | "MAURITIUS" => "MUS",
        "塞舌尔" | "SEYCHELLES" => "SYC",
        "科摩罗" | "COMOROS" => "COM",
        "南非" | "SOUTH AFRICA" => "ZAF",
        "尼日利亚" | "NIGERIA" => "NGA",
        "加纳" | "GHANA" => "GHA",
        "塞内加尔" | "SENEGAL" => "SEN",
        "马里" | "MALI" => "MLI",
        "布基纳法索" | "BURKINA FASO" => "BFA",
        "尼日尔" | "NIGER" => "NER",
        "贝宁" | "BENIN" => "BEN",
        "多哥" | "TOGO" => "TGO",
        "科特迪瓦" | "CÔTE D'IVOIRE" | "COTE D'IVOIRE" | "IVORY COAST" => "CIV",
        "利比里亚" | "LIBERIA" => "LBR",
        "塞拉利昂" | "SIERRA LEONE" => "SLE",
        "几内亚" | "GUINEA" => "GIN",
        "几内亚比绍" | "GUINEA-BISSAU" => "GNB",
        "冈比亚" | "GAMBIA" => "GMB",
        "佛得角" | "CABO VERDE" | "CAPE VERDE" => "CPV",
        "喀麦隆" | "CAMEROON" => "CMR",
        "中非" | "CENTRAL AFRICAN REPUBLIC" => "CAF",
        "乍得" | "CHAD" => "TCD",
        "赤道几内亚" | "EQUATORIAL GUINEA" => "GNQ",
        "加蓬" | "GABON" => "GAB",
        "刚果(布)" | "刚果共和国" | "CONGO" | "REPUBLIC OF THE CONGO" | "CONGO-BRAZZAVILLE" => "COG",
        "刚果(金)" | "刚果民主共和国" | "DR CONGO" | "DEMOCRATIC REPUBLIC OF THE CONGO" | "CONGO-KINSHASA" => "COD",
        "安哥拉" | "ANGOLA" => "AGO",
        "圣多美和普林西比" | "SAO TOME AND PRINCIPE" => "STP",
        "巴西" | "BRAZIL" => "BRA",
        "阿根廷" | "ARGENTINA" => "ARG",
        "智利" | "CHILE" => "CHL",
        "秘鲁" | "PERU" => "PER",
        "哥伦比亚" | "COLOMBIA" => "COL",
        "委内瑞拉" | "VENEZUELA" => "VEN",
        "厄瓜多尔" | "ECUADOR" => "ECU",
        "玻利维亚" | "BOLIVIA" => "BOL",
        "巴拉圭" | "PARAGUAY" => "PRY",
        "乌拉圭" | "URUGUAY" => "URY",
        "圭亚那" | "GUYANA" => "GUY",
        "苏里南" | "SURINAME" => "SUR",
        "墨西哥" | "MEXICO" => "MEX",
        "危地马拉" | "GUATEMALA" => "GTM",
        "伯利兹" | "BELIZE" => "BLZ",
        "洪都拉斯" | "HONDURAS" => "HND",
        "萨尔瓦多" | "EL SALVADOR" => "SLV",
        "尼加拉瓜" | "NICARAGUA" => "NIC",
        "哥斯达黎加" | "COSTA RICA" => "CRI",
        "巴拿马" | "PANAMA" => "PAN",
        "古巴" | "CUBA" => "CUB",
        "牙买加" | "JAMAICA" => "JAM",
        "海地" | "HAITI" => "HTI",
        "多米尼加" | "DOMINICAN REPUBLIC" => "DOM",
        "特立尼达和多巴哥" | "TRINIDAD AND TOBAGO" => "TTO",
        "巴巴多斯" | "BARBADOS" => "BRB",
        "圣卢西亚" | "SAINT LUCIA" => "LCA",
        "格林纳达" | "GRENADA" => "GRD",
        "圣文森特和格林纳丁斯" | "SAINT VINCENT AND THE GRENADINES" => "VCT",
        "安提瓜和巴布达" | "ANTIGUA AND BARBUDA" => "ATG",
        "圣基茨和尼维斯" | "SAINT KITTS AND NEVIS" => "KNA",
        "多米尼克" | "DOMINICA" => "DMA",
        "巴哈马" | "BAHAMAS" => "BHS",
        "斐济" | "FIJI" => "FJI",
        "巴布亚新几内亚" | "PAPUA NEW GUINEA" => "PNG",
        "所罗门群岛" | "SOLOMON ISLANDS" => "SLB",
        "瓦努阿图" | "VANUATU" => "VUT",
        "萨摩亚" | "SAMOA" => "WSM",
        "汤加" | "TONGA" => "TON",
        "基里巴斯" | "KIRIBATI" => "KIR",
        "图瓦卢" | "TUVALU" => "TUV",
        "瑙鲁" | "NAURU" => "NRU",
        "帕劳" | "PALAU" => "PLW",
        "马绍尔群岛" | "MARSHALL ISLANDS" => "MHL",
        "密克罗尼西亚" | "MICRONESIA" | "FEDERATED STATES OF MICRONESIA" => "FSM",
        "库克群岛" | "COOK ISLANDS" => "COK",
        "纽埃" | "NIUE" => "NIU",
        "中国香港" | "香港" | "HONG KONG" | "HONGKONG" => "HKG",
        "中国台湾" | "台湾" | "TAIWAN" => "TWN",
        "中国澳门" | "澳门" | "MACAO" | "MACAU" => "MAC",
        "蒙古" | "MONGOLIA" => "MNG",
        "哈萨克斯坦" | "KAZAKHSTAN" => "KAZ",
        "乌兹别克斯坦" | "UZBEKISTAN" => "UZB",
        "土库曼斯坦" | "TURKMENISTAN" => "TKM",
        "吉尔吉斯斯坦" | "KYRGYZSTAN" => "KGZ",
        "塔吉克斯坦" | "TAJIKISTAN" => "TJK",
        "格鲁吉亚" | "GEORGIA" => "GEO",
        "亚美尼亚" | "ARMENIA" => "ARM",
        "阿塞拜疆" | "AZERBAIJAN" => "AZE",
        "列支敦士登" | "LIECHTENSTEIN" => "LIE",
        "摩纳哥" | "MONACO" => "MCO",
        "圣马力诺" | "SAN MARINO" => "SMR",
        "安道尔" | "ANDORRA" => "AND",
        "梵蒂冈" | "VATICAN" | "HOLY SEE" => "VAT",
        "黑山" | "MONTENEGRO" => "MNE",
        "波斯尼亚和黑塞哥维那" | "波黑" | "BOSNIA AND HERZEGOVINA" => "BIH",
        "北马其顿" | "NORTH MACEDONIA" | "MACEDONIA" => "MKD",
        "阿尔巴尼亚" | "ALBANIA" => "ALB",
        _ => "",
    };
    if !code.is_empty() {
        code.to_string()
    } else {
        // 未知值：返回原始值，让调用方处理错误
        trimmed.to_string()
    }
}

// ============================================================================
// Semantic Type Resolution (新增)
// ============================================================================

/// 解析语义路径并返回字段值
///
/// 路径格式："{section_ref}.semantic://{semantic_type}"
/// 示例："section_pet_dog.semantic://pet.name"、"宠物狗.semantic://pet.name"
fn extract_by_semantic_type(
    field_id: &str,
    json_value: &serde_json::Value,
    plugin_mappings: Option<&std::collections::HashMap<String, String>>,
) -> Option<String> {
    rust_log(&format!("[extract_by_semantic_type] START field_id={}", field_id));

    let (section_ref, semantic_type) = parse_semantic_path(field_id)?;
    rust_log(&format!("[extract_by_semantic_type] section_ref={}, semantic_type={}", section_ref, semantic_type));

    // 解析 section_ref 为 section_id
    let section_id = resolve_section_reference(section_ref, json_value)?;
    rust_log(&format!("[extract_by_semantic_type] resolved section_id={}", section_id));

    // 获取 section 对象
    let section = find_section_by_id(&section_id, json_value)?;

    // 获取所有 objects 数组引用
    let all_objects = json_value
        .get("unified_objects")
        .and_then(|uo| uo.get("objects"))
        .and_then(|arr| arr.as_array())?;

    // 优先使用插件级映射
    if let Some(mappings) = plugin_mappings {
        if let Some(machine_key) = mappings.get(semantic_type) {
            rust_log(&format!("[extract_by_semantic_type] using plugin mapping: {} -> {}", semantic_type, machine_key));
            return find_value_in_section_children(section, machine_key, all_objects)
                .or_else(|| find_value_in_section_self(section, machine_key));
        }
    }

    // 使用 section 的 __semanticTypes 查找机器 key
    let semantic_types = section.get("__semanticTypes").and_then(|st: &serde_json::Value| st.as_object())?;
    let machine_key = find_machine_key_by_semantic_type(semantic_types, semantic_type)?;
    rust_log(&format!("[extract_by_semantic_type] found machine_key={} for semantic_type={}", machine_key, semantic_type));

    // 在 section 的子对象或自身中查找值
    let result = find_value_in_section_children(section, &machine_key, all_objects)
        .or_else(|| find_value_in_section_self(section, &machine_key));
    rust_log(&format!("[extract_by_semantic_type] result={:?}", result.as_ref().map(|s| &s[..s.len().min(50)])));
    result
}

/// 解析语义路径为 (section_ref, semantic_type)
fn parse_semantic_path(field_id: &str) -> Option<(&str, &str)> {
    let parts: Vec<&str> = field_id.split(".semantic://").collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0], parts[1]))
}

/// 通过名称或 ID 解析 section 引用
fn resolve_section_reference(
    section_ref: &str,
    json_value: &serde_json::Value,
) -> Option<String> {
    // 1. 内置 section 映射（向后兼容）
    let built_in = match section_ref {
        "identity" | "Identity" => Some("__section_identity"),
        "contact" | "Contact" => Some("__section_contact"),
        "passport" | "Passport" => Some("__section_passport"),
        "bankAccount" | "Bank Account" | "bank_account" => Some("__section_bank_account"),
        _ => None,
    };
    if let Some(id) = built_in {
        return Some(id.to_string());
    }

    // 2. 如果 section_ref 看起来已经是 ID（以 __ 开头或包含 auto_），直接使用
    if section_ref.starts_with("__") || section_ref.starts_with("section_") {
        return Some(section_ref.to_string());
    }

    // 3. 动态 section 查找：遍历 objects，匹配 name
    let objects = json_value
        .get("unified_objects")
        .and_then(|uo| uo.get("objects"))
        .and_then(|arr| arr.as_array())?;

    let normalized_ref = section_ref.trim().to_lowercase();
    for obj in objects {
        let obj_name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let type_id = obj.get("typeId").and_then(|t| t.as_str()).unwrap_or("");
        if (type_id == "collection" || type_id == "page")
            && obj_name.trim().to_lowercase() == normalized_ref {
            return obj.get("id").and_then(|id| id.as_str()).map(|s| s.to_string());
        }
    }

    None
}

/// 在 objects 中通过 ID 查找 section
fn find_section_by_id<'a>(
    section_id: &str,
    json_value: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    let objects = json_value
        .get("unified_objects")
        .and_then(|uo| uo.get("objects"))
        .and_then(|arr| arr.as_array())?;

    objects.iter().find(|obj| {
        obj.get("id").and_then(|id| id.as_str()) == Some(section_id)
    })
}

/// 通过语义类型查找机器 key（取第一个匹配）
fn find_machine_key_by_semantic_type(
    semantic_types: &serde_json::Map<String, serde_json::Value>,
    target: &str,
) -> Option<String> {
    for (key, value) in semantic_types {
        if value.as_str() == Some(target) {
            return Some(key.clone());
        }
    }
    None
}

/// 获取字段的实际敏感度
fn get_field_sensitivity_value(
    section: &serde_json::Value,
    machine_key: &str,
) -> SensitivityLevel {
    section
        .get("properties")
        .and_then(|p| p.as_object())
        .and_then(|props| props.get(machine_key))
        .and_then(|prop| prop.get("sensitivity"))
        .and_then(|s| s.as_str())
        .and_then(|s| SensitivityLevel::from_str(s))
        .unwrap_or(SensitivityLevel::Public)
}

/// 在 section 的子对象中查找指定 key 的值
fn find_value_in_section_children(
    section: &serde_json::Value,
    key: &str,
    all_objects: &[serde_json::Value],
) -> Option<String> {
    let child_ids: Vec<&str> = section
        .get("childrenIds")
        .and_then(|c| c.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    for obj in all_objects {
        let obj_id = obj.get("id").and_then(|id| id.as_str()).unwrap_or("");
        if !child_ids.contains(&obj_id) {
            continue;
        }
        if obj.get("isDeleted").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }

        if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
            if let Some(prop) = props.get(key) {
                if let Some(text) = prop.get("text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
                if let Some(value) = prop.get("value").and_then(|v| v.as_f64()) {
                    return Some(value.to_string());
                }
                if let Some(iso_date) = prop.get("isoDate").and_then(|d| d.as_str()) {
                    if !iso_date.is_empty() {
                        return Some(iso_date.to_string());
                    }
                }
                if let Some(checked) = prop.get("checked").and_then(|c| c.as_bool()) {
                    return Some(checked.to_string());
                }
            }
        }
    }

    None
}

/// 在 section 自身的 properties 中查找（用于 section 直接存储数据的场景）
fn find_value_in_section_self(section: &serde_json::Value, key: &str) -> Option<String> {
    let props = section.get("properties").and_then(|p| p.as_object())?;
    let prop = props.get(key)?;

    if let Some(text) = prop.get("text").and_then(|t| t.as_str()) {
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    if let Some(value) = prop.get("value").and_then(|v| v.as_f64()) {
        return Some(value.to_string());
    }
    if let Some(iso_date) = prop.get("isoDate").and_then(|d| d.as_str()) {
        if !iso_date.is_empty() {
            return Some(iso_date.to_string());
        }
    }
    if let Some(checked) = prop.get("checked").and_then(|c| c.as_bool()) {
        return Some(checked.to_string());
    }

    None
}

/// 【新增 Host Function】获取包含指定语义类型的所有 section
fn get_sections_with_semantic_type(
    semantic_type: &str,
    json_value: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let mut results = Vec::new();

    let empty_vec: Vec<serde_json::Value> = Vec::new();
    let objects = json_value
        .get("unified_objects")
        .and_then(|uo| uo.get("objects"))
        .and_then(|arr| arr.as_array())
        .unwrap_or(&empty_vec);

    for obj in objects {
        let type_id = obj.get("typeId").and_then(|t| t.as_str()).unwrap_or("");
        if type_id != "collection" && type_id != "page" {
            continue;
        }

        if let Some(st_map) = obj.get("__semanticTypes").and_then(|st: &serde_json::Value| st.as_object()) {
            for (_, value) in st_map {
                if value.as_str() == Some(semantic_type) {
                    results.push(serde_json::json!({
                        "section_id": obj.get("id").and_then(|id| id.as_str()).unwrap_or(""),
                        "section_name": obj.get("name").and_then(|n| n.as_str()).unwrap_or(""),
                    }));
                    break;
                }
            }
        }
    }

    results
}

async fn proxy_http_post(url: &str, body: &str) -> Result<String, HttpError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|_| HttpError::Network)?;

    let response = client
        .post(url)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                HttpError::Timeout
            } else {
                HttpError::Network
            }
        })?;

    let text = response.text().await.map_err(|_| HttpError::Network)?;
    Ok(text)
}

// ============================================================================
// HTTP Error
// ============================================================================

#[derive(Debug)]
pub enum HttpError {
    Timeout,
    Network,
}

// ============================================================================
// Async helper
// ============================================================================

/// 在当前或新创建的 tokio runtime 中阻塞执行 async 代码
fn block_on_http<F>(fut: F) -> Result<String, HttpError>
where
    F: std::future::Future<Output = Result<String, HttpError>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(fut),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().map_err(|_| HttpError::Network)?;
            rt.block_on(fut)
        }
    }
}

// ============================================================================
// URL / Domain helpers
// ============================================================================

fn extract_host(url: &str) -> String {
    url.split("//")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').next())
        .unwrap_or("")
        .to_string()
}

// ============================================================================
// Wasm memory helpers
// ============================================================================

fn read_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, len: usize) -> String {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else {
        return String::new();
    };
    let mut buf = vec![0u8; len];
    memory.read(caller, ptr, &mut buf).unwrap_or(());
    String::from_utf8_lossy(&buf).to_string()
}

/// 计算 JSON Value 的嵌套深度
fn json_depth(val: &serde_json::Value) -> usize {
    match val {
        serde_json::Value::Object(map) => {
            if map.is_empty() {
                1
            } else {
                1 + map.values().map(json_depth).max().unwrap_or(0)
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                1
            } else {
                1 + arr.iter().map(json_depth).max().unwrap_or(0)
            }
        }
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_uom_versioned_wrapper() {
        let json = serde_json::json!({
            "version": 1,
            "data": {
                "unified_objects": {
                    "objects": [
                        {
                            "typeId": "profile_address",
                            "name": "Home",
                            "isDeleted": false,
                            "properties": {
                                "street": {"type": "text", "text": "123 Main St"},
                                "city": {"type": "text", "text": "Springfield"},
                                "country": {"type": "text", "text": "US"}
                            }
                        }
                    ]
                },
                "schema_version": 4
            }
        });

        assert_eq!(
            extract_from_unified_object_model("address.count", &json),
            Some("1".to_string())
        );
        assert_eq!(
            extract_from_unified_object_model("address[0].street", &json),
            Some("123 Main St".to_string())
        );
        assert_eq!(
            extract_from_unified_object_model("address[0].city", &json),
            Some("Springfield".to_string())
        );
        assert_eq!(
            extract_from_unified_object_model("address[0].country", &json),
            Some("US".to_string())
        );
    }

    #[test]
    fn test_extract_from_uom_direct_profile_data() {
        let json = serde_json::json!({
            "unified_objects": {
                "objects": [
                    {
                        "typeId": "profile_address",
                        "name": "Home",
                        "isDeleted": false,
                        "properties": {
                            "street": {"type": "text", "text": "456 Oak Ave"},
                            "city": {"type": "text", "text": "Metro City"},
                            "country": {"type": "text", "text": "CN"}
                        }
                    }
                ]
            },
            "schema_version": 4
        });

        assert_eq!(
            extract_from_unified_object_model("address.count", &json),
            Some("1".to_string())
        );
        assert_eq!(
            extract_from_unified_object_model("address[0].street", &json),
            Some("456 Oak Ave".to_string())
        );
        assert_eq!(
            extract_from_unified_object_model("address[0].country", &json),
            Some("CN".to_string())
        );
    }
}

fn write_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, value: &str) {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else { return };
    memory.write(&mut *caller, ptr, value.as_bytes()).unwrap_or(());
    // 追加 null terminator，SDK 侧通过查找 \0 确定字符串长度
    memory.write(&mut *caller, ptr + value.len(), &[0]).unwrap_or(());
}
