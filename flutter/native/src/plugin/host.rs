//! Host functions - Secure interface between Wasm and Rust Core
//!
//! Plugins can only access data through these strictly defined functions.
//! All access is logged, rate-limited, and subject to user consent.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use wasmtime::{Caller, Linker};

use super::manifest::PluginManifest;
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
    pub response: oneshot::Sender<ConsentResult>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityLevel {
    /// 公开数据，无需确认
    Public,
    /// 内部数据，无需确认
    Internal,
    /// 敏感数据，需要用户确认
    Sensitive,
    /// 关键数据，需要用户确认
    Critical,
}

impl SensitivityLevel {
    /// 是否需要用户显式确认
    pub fn needs_confirmation(&self) -> bool {
        matches!(self, SensitivityLevel::Sensitive | SensitivityLevel::Critical)
    }
}

/// 根据字段路径解析敏感度（运行时映射表）
fn resolve_field_sensitivity(field_id: &str) -> SensitivityLevel {
    match field_id {
        "identity.full_name" => SensitivityLevel::Public,
        "identity.contact.emails" | "identity.contact.phones" => SensitivityLevel::Internal,
        "identity.id_card.number"
        | "travel.primary_passport.number"
        | "financial.primary_bank_account.number" => SensitivityLevel::Critical,
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
    pub rate_limiter: Arc<RateLimiter>,
    pub session_expires_at: Instant,
    pub wasi: wasmtime_wasi::preview1::WasiP1Ctx,
}

impl SoloHostFunctions {
    pub fn new(
        plugin_id: &str,
        plugin_name: &str,
        session_id: &str,
        manifest: PluginManifest,
        consent_tx: mpsc::Sender<ConsentRequest>,
        audit_tx: mpsc::Sender<AuditEntry>,
        rate_limiter: Arc<RateLimiter>,
        ttl_seconds: u64,
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
            rate_limiter,
            session_expires_at: Instant::now() + Duration::from_secs(ttl_seconds),
            wasi,
        }
    }

    /// 注册所有 Host Functions 到 Linker
    pub fn register(linker: &mut Linker<Self>) -> Result<(), String> {
        // solosoul_request_field(field_id_ptr, field_id_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "solosoul",
                "request_field",
                |mut caller: Caller<'_, Self>,
                 field_id_ptr: i32,
                 field_id_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> i32 {
                    let field_id =
                        read_memory(&mut caller, field_id_ptr as usize, field_id_len as usize);

                    // 使用独立作用域获取 funcs 数据，避免与后续 write_memory 的 mutable borrow 冲突
                    let (plugin_id, session_id, manifest, rate_limiter, consent_tx, audit_tx, plugin_name, session_expires_at) = {
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
                        )
                    };

                    // 1. 速率限制检查
                    if !rate_limiter.check(&plugin_id, &field_id) {
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
                    if !manifest.is_field_requested(&field_id) {
                        log_audit(
                            &audit_tx,
                            &plugin_id,
                            &session_id,
                            AuditAction::FieldAccessDenied {
                                field: field_id.clone(),
                            },
                        );
                        return -1; // PermissionDenied
                    }

                    // 3. 校验 Session 未过期
                    if Instant::now() > session_expires_at {
                        return -3; // TtlExpired
                    }

                    // 4. 根据敏感度决定是否需要用户确认
                    let sensitivity = resolve_field_sensitivity(&field_id);
                    let needs_confirmation = sensitivity.needs_confirmation();

                    if !needs_confirmation {
                        match decrypt_field_value(&field_id) {
                            Ok(value) => {
                                if value.len() >= out_cap as usize {
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
                                return 0;
                            }
                            Err(_) => return -5, // InvalidField
                        }
                    }

                    // 5. 敏感字段：通过 Flutter 通道请求用户确认
                    let request_id = uuid::Uuid::new_v4().to_string();
                    let (tx, rx) = oneshot::channel();
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
                    // 注意：若超时，Rust 侧的 oneshot::Receiver 被 drop，但 Flutter 端弹窗可能仍然存在。
                    // Dart 端应通过 PluginEvent::ConsentTimeout 关闭弹窗，防止状态泄漏。
                    match rx.blocking_recv() {
                        Ok(ConsentResult::Approved(value)) => {
                            if value.len() >= out_cap as usize {
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
                            0
                        }
                        Ok(ConsentResult::Denied) => -2,        // UserDenied
                        Ok(ConsentResult::Expired) | Err(_) => -3, // TtlExpired / Timeout
                    }
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_post_data(url_ptr, url_len, body_ptr, body_len, out_ptr, out_cap) -> i32
        linker
            .func_wrap(
                "solosoul",
                "post_data",
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
                            if data.len() >= out_cap as usize {
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
                "solosoul",
                "log",
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
                },
            )
            .map_err(|e| e.to_string())?;

        // solosoul_get_timestamp() -> i64
        linker
            .func_wrap(
                "solosoul",
                "get_timestamp",
                |_caller: Caller<'_, Self>| -> i64 {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64
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
fn decrypt_field_value(field_id: &str) -> Result<String, String> {
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
    let profile_summary = profiles.first().ok_or("No profiles found")?;

    // 5. 加载 profile 的加密 data
    let profile = vault_store
        .load_profile(&profile_summary.id)
        .map_err(|e| format!("Failed to load profile: {}", e))?
        .ok_or("Profile not found")?;

    // 6. 解密
    let plaintext = crate::crypto::decrypt_profile_data(&session_key, &profile.data)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    // 7. 反序列化
    let json_str = String::from_utf8_lossy(&plaintext);
    let versioned: VersionedProfileData = serde_json::from_str(&json_str)
        .map_err(|e| format!("Failed to parse profile data: {}", e))?;

    // 8. 按字段路径取值
    extract_field_value(field_id, &versioned.data)
        .ok_or_else(|| format!("Field '{}' not found or empty", field_id))
}

/// 从 ProfileData 中按字段路径提取值
fn extract_field_value(field_id: &str, data: &ProfileData) -> Option<String> {
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

fn write_memory(caller: &mut Caller<'_, SoloHostFunctions>, ptr: usize, value: &str) {
    let memory = caller.get_export("memory").and_then(|e| e.into_memory());
    let Some(memory) = memory else { return };
    memory.write(caller, ptr, value.as_bytes()).unwrap_or(());
}
